package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID

/** SVC's group state in memory. Calls change state only; events are driven by the test. */
class FakeSvcGroups : SvcGroupsPort {
    var enabled = true
    val groups = LinkedHashMap<UUID, SvcGroupView>()
    val persistent = HashSet<UUID>()
    val memberOf = HashMap<UUID, UUID>()
    val cancelJoinsFor = HashSet<UUID>()
    var removeFailures = 0
    val removeCalls = ArrayList<UUID>()
    val calls = ArrayList<String>()

    // The server thread did not answer in time: every call throws, as SvcApiGroups does.
    var unavailable = false

    // The create runs on the server thread after the caller has already given up waiting.
    var createLandsButTimesOut = false

    fun add(name: String, type: SvcGroupType = SvcGroupType.OPEN, password: Boolean = false, hidden: Boolean = false): SvcGroupView {
        val view = SvcGroupView(UUID.randomUUID(), name, type, password, hidden)
        groups[view.id] = view
        return view
    }

    private fun answer() {
        if (unavailable) throw SvcUnavailableException()
    }

    override fun groupsEnabled(): Boolean {
        answer()
        return enabled
    }

    override fun groups(): List<SvcGroupView> {
        answer()
        return groups.values.toList()
    }

    override fun membersOf(group: UUID): List<UUID> {
        answer()
        return memberOf.filterValues { it == group }.keys.toList()
    }

    override fun create(name: String, type: SvcGroupType, persistent: Boolean): UUID? {
        answer()
        val view = add(name, type)
        if (persistent) this.persistent.add(view.id)
        calls.add("create:$name")
        if (createLandsButTimesOut) throw SvcUnavailableException()
        return view.id
    }

    override fun setGroup(player: UUID, group: UUID?): Boolean {
        answer()
        calls.add("setGroup:$player:$group")
        if (group != null && player in cancelJoinsFor) return false
        if (group == null) memberOf.remove(player) else memberOf[player] = group
        return true
    }

    override fun remove(group: UUID): Boolean {
        answer()
        removeCalls.add(group)
        calls.add("remove:$group")
        if (memberOf.containsValue(group) || removeFailures > 0) {
            if (removeFailures > 0) removeFailures--
            return false
        }
        groups.remove(group)
        return true
    }
}
