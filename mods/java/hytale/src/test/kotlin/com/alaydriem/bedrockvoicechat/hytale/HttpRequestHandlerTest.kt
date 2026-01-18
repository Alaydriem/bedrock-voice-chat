package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.dto.*
import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import org.junit.jupiter.api.*
import org.junit.jupiter.api.Assertions.*

/**
 * Integration tests for HttpRequestHandler using MockBvcServer.
 */
class HttpRequestHandlerTest {
    private lateinit var mockServer: MockBvcServer
    private lateinit var handler: HttpRequestHandler

    @BeforeEach
    fun setUp() {
        mockServer = MockBvcServer(18080)
        mockServer.start()
        handler = HttpRequestHandler(mockServer.getBaseUrl(), "test-token")
    }

    @AfterEach
    fun tearDown() {
        mockServer.stop()
    }

    @Test
    fun `should send payload to server`() {
        // Given
        val players = listOf(
            PlayerData(
                name = "TestPlayer",
                x = 100.0, y = 64.0, z = 200.0,
                yaw = 90.0f, pitch = 0.0f,
                dimension = Dimension.Hytale.ORBIS,
                worldUuid = "test-world-uuid"
            )
        )
        val payload = Payload(GameType.HYTALE, players)

        // When
        handler.sendAsync(payload)

        // Then - wait a bit for async request
        Thread.sleep(500)

        val receivedPayloads = mockServer.getReceivedPayloads()
        assertEquals(1, receivedPayloads.size)
        
        val json = receivedPayloads[0]
        assertTrue(json.contains("\"game\":\"hytale\""))
        assertTrue(json.contains("\"name\":\"TestPlayer\""))
        assertTrue(json.contains("\"world_uuid\":\"test-world-uuid\""))
    }

    @Test
    fun `should send multiple payloads`() {
        // Given
        val players = listOf(
            PlayerData(
                name = "Player1",
                x = 0.0, y = 64.0, z = 0.0,
                yaw = 0.0f, pitch = 0.0f,
                dimension = Dimension.Minecraft.OVERWORLD,
                deafen = false
            ),
            PlayerData(
                name = "Player2",
                x = 100.0, y = 64.0, z = 100.0,
                yaw = 180.0f, pitch = 0.0f,
                dimension = Dimension.Minecraft.OVERWORLD,
                deafen = true
            )
        )

        // When
        handler.sendAsync(Payload(GameType.MINECRAFT, players))
        handler.sendAsync(Payload(GameType.MINECRAFT, players))

        // Then
        Thread.sleep(500)
        assertEquals(2, mockServer.getReceivedPayloads().size)
    }
}
