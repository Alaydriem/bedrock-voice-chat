package com.alaydriem.bedrockvoicechat.svc.link

import com.alaydriem.bedrockvoicechat.svc.SvcGroupViews
import de.maxhenkel.voicechat.api.Group
import de.maxhenkel.voicechat.api.VoicechatServerApi
import org.slf4j.LoggerFactory
import java.util.UUID
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit

/**
 * [SvcGroupsPort] over Simple Voice Chat's server API.
 *
 * Every call runs on the server thread and the caller waits for it. The linker calls from its own
 * thread, and the server thread never waits on that thread, so the two cannot deadlock. A call
 * that times out, fails, or that a stopping server refuses throws [SvcUnavailableException]: its
 * answer is unknown, and no default stands in for it.
 *
 * [refresh] takes one snapshot per tick of who is in which group, who runs the SVC mod, and each
 * player's name. Membership and names are read from it, so a resync costs a few hops rather than
 * several per player, and the audio path never calls SVC off the server thread.
 *
 * Names SVC types, so it loads only once SVC is known to be present.
 */
class SvcApiGroups(
    private val api: VoicechatServerApi,
    private val onlinePlayers: () -> List<UUID>,
    private val onServerThread: (Runnable) -> Unit,
    private val nameOf: (UUID) -> String?,
) : SvcGroupsPort {
    private class Snapshot(
        val members: Map<UUID, List<UUID>>,
        val groupOf: Map<UUID, SvcGroupView>,
        val installed: Set<UUID>,
        val names: Map<UUID, String>,
    )

    @Volatile
    private var snapshot = Snapshot(emptyMap(), emptyMap(), emptySet(), emptyMap())

    fun <T> onServer(block: () -> T): T {
        val result = CompletableFuture<T>()
        try {
            onServerThread(Runnable {
                try {
                    result.complete(block())
                } catch (e: Exception) {
                    result.completeExceptionally(e)
                }
            })
            return result.get(CALL_TIMEOUT_MS, TimeUnit.MILLISECONDS)
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            throw SvcUnavailableException()
        } catch (e: Exception) {
            logger.debug("Simple Voice Chat group call did not answer: {}", e.toString())
            throw SvcUnavailableException()
        }
    }

    /** Keeps the previous snapshot when SVC does not answer. */
    fun refresh() {
        snapshot = try {
            onServer {
                val members = HashMap<UUID, MutableList<UUID>>()
                val groupOf = HashMap<UUID, SvcGroupView>()
                val installed = HashSet<UUID>()
                val names = HashMap<UUID, String>()
                for (player in onlinePlayers()) {
                    nameOf(player)?.let { names[player] = it }
                    val connection = api.getConnectionOf(player) ?: continue
                    if (connection.isInstalled) installed.add(player)
                    val group = connection.group ?: continue
                    members.getOrPut(group.id) { ArrayList() }.add(player)
                    groupOf[player] = SvcGroupViews.of(group)
                }
                Snapshot(members, groupOf, installed, names)
            }
        } catch (e: SvcUnavailableException) {
            snapshot
        }
    }

    fun isInstalled(player: UUID): Boolean = player in snapshot.installed

    fun groupOf(player: UUID): SvcGroupView? = snapshot.groupOf[player]

    fun nameOf(player: UUID): String? = snapshot.names[player]

    override fun groupsEnabled(): Boolean = onServer { api.serverConfig.getBoolean("enable_groups", true) }

    override fun groups(): List<SvcGroupView> = onServer { api.groups.map(SvcGroupViews::of) }

    // From the snapshot, at most one tick old. A member who joined since makes removeGroup refuse,
    // which the linker retries on its next pass.
    override fun membersOf(group: UUID): List<UUID> = snapshot.members[group] ?: emptyList()

    override fun create(name: String, type: SvcGroupType, persistent: Boolean): UUID? = onServer {
        api.groupBuilder()
            .setName(name)
            .setType(
                when (type) {
                    SvcGroupType.OPEN -> Group.Type.OPEN
                    SvcGroupType.NORMAL -> Group.Type.NORMAL
                    SvcGroupType.ISOLATED -> Group.Type.ISOLATED
                }
            )
            .setPersistent(persistent)
            .build()
            ?.id
    }

    // Read back, because a join another plugin cancelled leaves the player where they were. From a
    // fresh connection: a VoicechatConnection is a snapshot, so the one just written to still
    // reports the old group, and every join would read as refused.
    override fun setGroup(player: UUID, group: UUID?): Boolean = onServer {
        val connection = api.getConnectionOf(player) ?: return@onServer false
        connection.group = group?.let { api.getGroup(it) ?: return@onServer false }
        api.getConnectionOf(player)?.group?.id == group
    }

    override fun remove(group: UUID): Boolean = onServer { api.removeGroup(group) }

    companion object {
        private const val CALL_TIMEOUT_MS: Long = 5000

        private val logger = LoggerFactory.getLogger("BVC Group Link")
    }
}
