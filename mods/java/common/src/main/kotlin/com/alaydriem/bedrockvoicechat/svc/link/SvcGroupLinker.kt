package com.alaydriem.bedrockvoicechat.svc.link

import com.alaydriem.bedrockvoicechat.control.GroupLinkApi
import com.alaydriem.bedrockvoicechat.control.GroupLinkEventJson
import com.alaydriem.bedrockvoicechat.control.GroupLinkEventKind
import com.alaydriem.bedrockvoicechat.control.LinkedChannelJson
import org.slf4j.LoggerFactory
import java.util.UUID

/**
 * Keeps each linked SVC group and BVC channel the same group.
 *
 * Every method runs on the link thread, one at a time. SVC events reach it as values posted from
 * the server thread; BVC events arrive by [pollBvc]. Both ports block, which is why none of this
 * runs on the server thread.
 *
 * Ownership: a player's membership belongs to their own side ([PlayerSides]), and a group's
 * existence belongs to the side it was made on ([LinkEntry.Origin]). The other side is corrected
 * to match.
 *
 * [actorIdOf] gives a player's in-game name, which BVC resolves like any actor. [playerOf] maps a
 * gamertag from BVC back to an online player.
 */
class SvcGroupLinker(
    private val svc: SvcGroupsPort,
    private val bvc: GroupLinkApi,
    private val table: SvcLinkTable,
    private val sides: PlayerSides,
    private val echo: EchoGuard,
    private val refusals: LinkRefusals,
    private val actorIdOf: (UUID) -> String?,
    private val playerOf: (String) -> UUID?,
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Group Link")
    }

    // The last BVC event applied. Zero asks for everything the server still holds.
    private var after = 0L

    // The server start the cursor belongs to. A new one means the sequence started over.
    private var epoch: Long? = null

    // The bridge's own creates need no echo key: the create call returns the id, and the table
    // holds it before the event the call fired is processed. A name key would swallow a player's
    // own group of the same name. A group with no creator was made by another plugin; resync
    // links it under its first member.
    fun onSvcCreate(group: SvcGroupView, creator: UUID?) {
        if (table.bySvc(group.id) != null) return
        if (!group.linkable) return
        val creatorName = creator?.let(actorIdOf) ?: return
        linkNewSvcGroup(group, creatorName)
    }

    fun onSvcJoin(group: UUID, player: UUID) {
        if (echo.consume("svc:join:$player:$group")) return
        if (sides.of(player) != PlayerSide.SVC) return
        val name = actorIdOf(player) ?: return
        val entry = table.bySvc(group)
        if (entry == null) {
            // Moving into a group that is not linked takes them out of the linked one.
            leaveBvc(name)
            return
        }
        joinBvc(name, entry.bvcChannel)
    }

    fun onSvcLeave(player: UUID, group: UUID?) {
        if (echo.consume("svc:leave:$player")) return
        if (sides.of(player) != PlayerSide.SVC) return
        if (group != null && table.bySvc(group) == null) return
        leaveBvc(actorIdOf(player) ?: return)
    }

    fun onSvcRemove(group: UUID) {
        if (echo.consume("svc:remove:$group")) return
        val entry = table.bySvc(group) ?: return
        // BVC owns a BVC-origin group, so resync puts its SVC side back.
        if (entry.origin == LinkEntry.Origin.BVC) return
        table.remove(entry)
        echo.expect("bvc:delete:${entry.bvcChannel}")
        bvc.close(entry.bvcChannel)
    }

    /**
     * The cursor moves past an event only once it is applied. An event SVC did not answer for is
     * read again on the next poll.
     */
    fun pollBvc() {
        val drain = bvc.drain(after) ?: return
        val restarted = epoch != null && epoch != drain.epoch
        epoch = drain.epoch
        if (drain.reset || restarted) {
            after = drain.events.lastOrNull()?.seq ?: 0
            resync()
            return
        }
        try {
            for (event in drain.events) {
                onBvcEvent(event)
                after = event.seq
            }
        } catch (e: SvcUnavailableException) {
            logger.debug("Simple Voice Chat did not answer; the rest of the BVC events wait")
        }
    }

    /**
     * Resync builds the starting state, so the feed's history is skipped rather than replayed on
     * top of it: old events would move players through groups that ended long ago.
     */
    fun start() {
        bvc.drain(after)?.let { drain ->
            epoch = drain.epoch
            after = drain.events.lastOrNull()?.seq ?: after
        }
        resync()
    }

    /**
     * A pass SVC does not answer changes nothing. "No answer" is not "no groups", and acting on it
     * would close or duplicate every linked group.
     */
    fun resync() {
        try {
            resyncPass()
        } catch (e: SvcUnavailableException) {
            logger.info("Simple Voice Chat did not answer; skipping this group resync")
        }
    }

    private fun resyncPass() {
        if (!svc.groupsEnabled()) {
            table.all().forEach(table::remove)
            return
        }
        retryPendingRemoves()

        val channels = bvc.channels()?.associateBy { it.id } ?: return
        val groups = svc.groups().associateBy { it.id }

        // Everything linked when the pass began is settled by reconcileEntry, including what it
        // closes or tears down; only the rest is a leftover.
        val known = table.all()
        for (entry in known) {
            reconcileEntry(entry, groups, channels)
        }
        val knownGroups = known.map { it.svcGroup }.toSet()
        val knownChannels = known.map { it.bvcChannel }.toSet()
        linkLeftovers(
            groups.values.filterNot { it.id in knownGroups },
            channels.values.filterNot { it.id in knownChannels },
        )

        val latest = bvc.channels()?.associateBy { it.id } ?: return
        for (entry in table.all().filterNot { it.pendingRemove }) {
            latest[entry.bvcChannel]?.let { correctMembers(entry, it) }
        }
    }

    /**
     * A lease stands in for Java members, who hold no BVC client. A linked group with none is left
     * to BVC's reaper, as an unlinked one is, and its Delete takes the SVC side with it.
     */
    fun renewLeases() {
        try {
            val channels = table.all()
                .filterNot { it.pendingRemove }
                .filter { entry -> svc.membersOf(entry.svcGroup).any { sides.of(it) == PlayerSide.SVC } }
                .map { it.bvcChannel }
            if (channels.isNotEmpty()) {
                bvc.lease(channels)
            }
        } catch (e: SvcUnavailableException) {
            logger.debug("Simple Voice Chat did not answer; leases renew on the next pass")
        }
    }

    private fun onBvcEvent(event: GroupLinkEventJson) {
        when (event.kind) {
            GroupLinkEventKind.Create -> onBvcCreate(event)
            GroupLinkEventKind.Join -> onBvcJoin(event)
            GroupLinkEventKind.Leave -> onBvcLeave(event)
            GroupLinkEventKind.Delete -> {
                if (echo.consume("bvc:delete:${event.channel}")) return
                table.byBvc(event.channel)?.let(::onBvcSideGone)
            }
            GroupLinkEventKind.Protected -> table.byBvc(event.channel)?.let(::tearDownSvc)
        }
    }

    private fun onBvcCreate(event: GroupLinkEventJson) {
        val name = event.channelName ?: return
        if (table.byBvc(event.channel) != null) return
        if (!svc.groupsEnabled()) return
        // A create SVC did not answer for may still have run; link that group, not a second one.
        val existing = svc.groups().firstOrNull {
            it.name == name && !it.membersOnly && it.linkable && table.bySvc(it.id) == null
        }
        if (existing != null) {
            table.put(LinkEntry(existing.id, event.channel, membersOnly = false, origin = LinkEntry.Origin.BVC))
            return
        }
        // The creator's own Join event, which follows, puts them in the group.
        linkNewBvcChannel(event.channel, name)
    }

    // The BVC side of a linked group ended. BVC owns a BVC-origin group, so its SVC side goes too.
    // SVC owns an SVC-origin group, so it gets a new BVC side: the channel was lost (a BVC restart,
    // a lapsed lease), not ended by the Java players who made it.
    private fun onBvcSideGone(entry: LinkEntry) {
        if (entry.origin == LinkEntry.Origin.BVC) {
            tearDownSvc(entry)
            return
        }
        val group = svc.groups().firstOrNull { it.id == entry.svcGroup }
        if (group == null) {
            table.remove(entry)
            return
        }
        relinkSvcGroup(entry, group)
    }

    private fun relinkSvcGroup(entry: LinkEntry, group: SvcGroupView) {
        val members = svc.membersOf(group.id)
        val creatorName = members.firstNotNullOfOrNull(actorIdOf)
        if (creatorName == null) {
            // Nobody left to own it; SVC's own cleanup ends an empty group.
            table.remove(entry)
            return
        }
        // Kept on failure, so the next resync tries again.
        val channel = bvc.createLinked(creatorName, group.name, entry.membersOnly) ?: return
        table.put(entry.copy(bvcChannel = channel))
        for (member in members) {
            if (sides.of(member) == PlayerSide.SVC) {
                joinBvc(actorIdOf(member) ?: continue, channel)
            }
        }
    }

    private fun onBvcJoin(event: GroupLinkEventJson) {
        val player = event.player ?: return
        if (echo.consume("bvc:join:$player:${event.channel}")) return
        val uuid = playerOf(player) ?: return
        refusals.clear(uuid)
        val entry = table.byBvc(event.channel) ?: return
        if (sides.of(uuid) == PlayerSide.BVC) {
            setSvcGroup(uuid, entry.svcGroup)
        }
    }

    private fun onBvcLeave(event: GroupLinkEventJson) {
        val player = event.player ?: return
        if (echo.consume("bvc:leave:$player")) return
        val uuid = playerOf(player) ?: return
        refusals.clear(uuid)
        if (table.byBvc(event.channel) == null) return
        if (sides.of(uuid) == PlayerSide.BVC) {
            clearSvcGroup(uuid)
        }
    }

    private fun linkNewSvcGroup(group: SvcGroupView, creatorName: String) {
        val channel = bvc.createLinked(creatorName, group.name, group.membersOnly) ?: return
        table.put(LinkEntry(group.id, channel, group.membersOnly, LinkEntry.Origin.SVC))
    }

    private fun linkNewBvcChannel(channel: String, name: String): UUID? {
        // Persistent, so SVC's cleanup of empty groups cannot end a group BVC owns.
        val group = svc.create(name, SvcGroupType.OPEN, persistent = true) ?: return null
        table.put(LinkEntry(group, channel, membersOnly = false, origin = LinkEntry.Origin.BVC))
        return group
    }

    private fun reconcileEntry(
        entry: LinkEntry,
        groups: Map<UUID, SvcGroupView>,
        channels: Map<String, LinkedChannelJson>,
    ) {
        val group = groups[entry.svcGroup]
        val channel = channels[entry.bvcChannel]
        when {
            channel == null && group == null -> table.remove(entry)
            channel == null && entry.origin == LinkEntry.Origin.SVC -> relinkSvcGroup(entry, group!!)
            channel == null -> tearDownSvc(entry)
            group == null && entry.origin == LinkEntry.Origin.BVC -> {
                table.remove(entry)
                linkNewBvcChannel(channel.id, channel.name)
            }
            group == null -> {
                table.remove(entry)
                echo.expect("bvc:delete:${entry.bvcChannel}")
                bvc.close(entry.bvcChannel)
            }
        }
    }

    // Groups neither side has linked: a missed create, a create SVC did not answer for, or what
    // was there before the bridge started. The lists are taken before any link is made, so a
    // group made here is not linked twice.
    //
    // Same-named groups pair up only when their members-only flags agree: a BVC channel's flag is
    // fixed when it is made, so a private SVC group paired with an open channel would be heard
    // nearby on the BVC side.
    private fun linkLeftovers(groups: Collection<SvcGroupView>, channels: Collection<LinkedChannelJson>) {
        val svcLeft = groups.filter { it.linkable && table.bySvc(it.id) == null }.toMutableList()
        val bvcLeft = channels.filter { table.byBvc(it.id) == null }.toMutableList()

        for (group in svcLeft.toList()) {
            val channel = bvcLeft.firstOrNull { it.name == group.name && it.membersOnly == group.membersOnly }
                ?: continue
            table.put(LinkEntry(group.id, channel.id, group.membersOnly, LinkEntry.Origin.SVC))
            svcLeft.remove(group)
            bvcLeft.remove(channel)
        }

        for (group in svcLeft) {
            val creatorName = svc.membersOf(group.id).firstNotNullOfOrNull(actorIdOf) ?: continue
            linkNewSvcGroup(group, creatorName)
        }
        for (channel in bvcLeft) {
            if (channel.membersOnly) {
                // Only the bridge makes a members-only channel, for a private SVC group. With no
                // such group left it has ended; re-creating it as an OPEN SVC group would open a
                // private group to every Java player and to nearby listeners.
                echo.expect("bvc:delete:${channel.id}")
                bvc.close(channel.id)
                continue
            }
            linkNewBvcChannel(channel.id, channel.name)
        }
    }

    private fun correctMembers(entry: LinkEntry, channel: LinkedChannelJson) {
        val inSvc = svc.membersOf(entry.svcGroup).toSet()
        val inBvc = channel.players.mapNotNull(playerOf).toSet()

        for (player in inSvc + inBvc) {
            when (sides.of(player)) {
                PlayerSide.SVC -> {
                    val name = actorIdOf(player) ?: continue
                    if (player in inSvc && player !in inBvc) joinBvc(name, entry.bvcChannel)
                    if (player in inBvc && player !in inSvc) leaveBvc(name)
                }
                PlayerSide.BVC -> {
                    if (player in inBvc && player !in inSvc) setSvcGroup(player, entry.svcGroup)
                    if (player in inSvc && player !in inBvc) clearSvcGroup(player)
                }
                PlayerSide.NONE -> Unit
            }
        }
    }

    private fun joinBvc(name: String, channel: String) {
        echo.expect("bvc:join:$name:$channel")
        bvc.join(name, channel)
    }

    private fun leaveBvc(name: String) {
        echo.expect("bvc:leave:$name")
        bvc.leave(name)
    }

    private fun setSvcGroup(player: UUID, group: UUID) {
        if (refusals.isRefused(player, group)) return
        echo.expect("svc:join:$player:$group")
        if (!svc.setGroup(player, group)) {
            refusals.refuse(player, group)
            logger.info("Another plugin refused a linked group join for {}; not retrying", player)
        }
    }

    private fun clearSvcGroup(player: UUID) {
        echo.expect("svc:leave:$player")
        svc.setGroup(player, null)
    }

    // SVC refuses removeGroup while any player state names the group, so every member is moved
    // out first.
    private fun tearDownSvc(entry: LinkEntry) {
        for (member in svc.membersOf(entry.svcGroup)) {
            clearSvcGroup(member)
        }
        echo.expect("svc:remove:${entry.svcGroup}")
        if (svc.remove(entry.svcGroup)) {
            table.remove(entry)
        } else {
            table.put(entry.copy(pendingRemove = true))
        }
    }

    private fun retryPendingRemoves() {
        for (entry in table.all().filter { it.pendingRemove }) {
            for (member in svc.membersOf(entry.svcGroup)) {
                clearSvcGroup(member)
            }
            echo.expect("svc:remove:${entry.svcGroup}")
            if (!svc.remove(entry.svcGroup)) {
                logger.warn("Could not remove linked SVC group {}; giving up", entry.svcGroup)
            }
            table.remove(entry)
        }
    }
}
