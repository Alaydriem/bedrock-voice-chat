package com.alaydriem.bedrockvoicechat.powernukkitx.commands

import com.alaydriem.bedrockvoicechat.admin.AdminAction
import com.alaydriem.bedrockvoicechat.admin.AdminConsole
import com.alaydriem.bedrockvoicechat.control.ControlAction
import com.alaydriem.bedrockvoicechat.control.ControlSendResult
import com.alaydriem.bedrockvoicechat.control.ControlSender
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Test
import org.mockito.Mockito.mockStatic
import org.mockito.kotlin.any
import org.mockito.kotlin.anyOrNull
import org.mockito.kotlin.doAnswer
import org.mockito.kotlin.doReturn
import org.mockito.kotlin.eq
import org.mockito.kotlin.mock
import org.mockito.kotlin.never
import org.mockito.kotlin.verify
import org.mockito.kotlin.whenever
import org.powernukkitx.Player
import org.powernukkitx.Server
import org.powernukkitx.command.CommandSender
import org.powernukkitx.command.ConsoleCommandSender
import org.powernukkitx.command.utils.CommandLogger
import org.powernukkitx.inventory.HumanInventory
import org.powernukkitx.item.Item

class BvcCommandTest {

    private val control = mock<ControlSender>()
    private val admin = mock<AdminConsole>()
    private val disc = mock<Item>()
    private val inventory = mock<HumanInventory> {
        on { addItem(any()) } doReturn emptyArray()
    }
    private val steve = mock<Player> {
        on { getName() } doReturn "Steve"
        on { hasPermission(any<String>()) } doReturn true
        on { getInventory() } doReturn inventory
    }
    private val discsCreated = mutableListOf<String>()

    private val command = BvcCommand(
        controlSender = control,
        adminConsole = admin,
        canonicalName = { it.getName() },
        onlinePlayers = { listOf(steve) },
        onServerThread = { it.run() },
        createDisc = { audioId -> discsCreated.add(audioId); disc },
    )

    @Test
    fun mute_deafen_and_record_send_their_toggle_as_the_player() {
        run(steve, "mute", "true")
        run(steve, "deafen", "false")
        run(steve, "record", "true")

        verify(control).send(eq(ControlAction.Mute(true)), eq("Steve"), anyOrNull())
        verify(control).send(eq(ControlAction.Deafen(false)), eq("Steve"), anyOrNull())
        verify(control).send(eq(ControlAction.Record(true)), eq("Steve"), anyOrNull())
    }

    @Test
    fun volume_and_hear_resolve_the_typed_target_to_an_online_player() {
        run(steve, "volume", "steve", "80")
        run(steve, "hear", "STEVE", "false")

        verify(control).send(eq(ControlAction.Volume("Steve", 80)), eq("Steve"), anyOrNull())
        verify(control).send(eq(ControlAction.Hear("Steve", false)), eq("Steve"), anyOrNull())
    }

    @Test
    fun an_unknown_target_is_refused_without_sending() {
        run(steve, "volume", "Alex", "80")

        verify(control, never()).send(any(), any(), anyOrNull())
        verify(steve).sendMessage("[BVC] Unknown player: Alex")
    }

    @Test
    fun a_volume_above_the_maximum_is_refused() {
        run(steve, "volume", "Steve", (ControlAction.MAX_LEVEL + 1).toString())

        verify(control, never()).send(any(), any(), anyOrNull())
    }

    @Test
    fun jukebox_mute_and_volume_target_the_jukebox() {
        run(steve, "jukebox", "mute", "true")
        run(steve, "jukebox", "volume", "50")

        verify(control).send(eq(ControlAction.Hear(ControlAction.JUKEBOX_TARGET, false)), eq("Steve"), anyOrNull())
        verify(control).send(eq(ControlAction.Volume(ControlAction.JUKEBOX_TARGET, 50)), eq("Steve"), anyOrNull())
    }

    @Test
    fun group_join_and_leave_send_their_action() {
        run(steve, "group", "join", "abc123")
        run(steve, "group", "leave")

        verify(control).send(eq(ControlAction.JoinGroup("abc123")), eq("Steve"), anyOrNull())
        verify(control).send(eq(ControlAction.LeaveGroup), eq("Steve"), anyOrNull())
    }

    @Test
    fun group_create_replies_with_the_share_code() {
        doAnswer { invocation ->
            invocation.getArgument<(ControlSendResult) -> Unit>(2)(ControlSendResult(true, "XK4-92Q"))
        }.whenever(control).send(eq(ControlAction.CreateGroup), eq("Steve"), anyOrNull())

        run(steve, "group", "create")

        verify(steve).sendMessage("[BVC] Group created — share code: XK4-92Q")
    }

    @Test
    fun a_control_subcommand_from_the_console_is_refused() {
        val console = mock<ConsoleCommandSender>()

        run(console, "mute", "true")

        verify(control, never()).send(any(), any(), anyOrNull())
        verify(console).sendMessage("This command must be run by a player")
    }

    @Test
    fun admin_from_the_console_runs_the_common_handler() {
        val console = mock<ConsoleCommandSender>()
        whenever(admin.run(AdminAction.GRANT, "Some Tag")).thenReturn("Granted BVC admin to Some Tag.")

        run(console, "admin", "grant", "Some Tag")

        verify(admin).run(AdminAction.GRANT, "Some Tag")
        verify(console).sendMessage("Granted BVC admin to Some Tag.")
    }

    @Test
    fun admin_from_a_player_is_refused() {
        run(steve, "admin", "revoke", "Alex")

        verify(admin, never()).run(any(), any())
    }

    @Test
    fun disc_gives_the_player_a_disc_for_the_whole_audio_id() {
        run(steve, "disc", "two", "words")

        assertEquals(listOf("two words"), discsCreated)
        verify(inventory).addItem(disc)
    }

    @Test
    fun disc_without_its_permission_is_refused() {
        whenever(steve.hasPermission("bvc.disc")).thenReturn(false)

        run(steve, "disc", "track")

        assertEquals(emptyList<String>(), discsCreated)
    }

    @Test
    fun give_hands_a_disc_to_the_named_player() {
        val server = mock<Server> {
            on { getPlayerByDisplayName("Steve") } doReturn steve
        }
        val console = mock<ConsoleCommandSender> {
            on { hasPermission(any<String>()) } doReturn true
        }

        mockStatic(Server::class.java).use { static ->
            static.`when`<Server> { Server.getInstance() }.thenReturn(server)
            run(console, "give", "Steve", "track")
        }

        assertEquals(listOf("track"), discsCreated)
        verify(inventory).addItem(disc)
    }

    private fun run(sender: CommandSender, vararg args: String) {
        val parsed = command.paramTree.matchAndParse(sender, "bvc", arrayOf(*args))
        assertNotNull(parsed, "`/bvc ${args.joinToString(" ")}` did not parse")
        command.execute(sender, "bvc", parsed!!, mock<CommandLogger>())
    }
}
