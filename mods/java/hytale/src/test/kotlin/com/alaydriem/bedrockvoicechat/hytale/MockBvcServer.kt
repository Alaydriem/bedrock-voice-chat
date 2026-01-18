package com.alaydriem.bedrockvoicechat.hytale

import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress

/**
 * Mock HTTP server for integration testing.
 * Captures payloads sent by the plugin.
 */
class MockBvcServer(private val port: Int = 8080) {
    private val receivedPayloads = mutableListOf<String>()
    private var server: HttpServer? = null

    /**
     * Start the mock server.
     */
    fun start() {
        server = HttpServer.create(InetSocketAddress(port), 0).apply {
            createContext("/api/position") { exchange ->
                val body = exchange.requestBody.bufferedReader().readText()
                synchronized(receivedPayloads) {
                    receivedPayloads.add(body)
                }

                // Send 200 OK response
                exchange.sendResponseHeaders(200, 0)
                exchange.responseBody.close()
            }
            start()
        }
        println("Mock BVC server started on port $port")
    }

    /**
     * Stop the mock server.
     */
    fun stop() {
        server?.stop(0)
        server = null
        println("Mock BVC server stopped")
    }

    /**
     * Get all received payloads.
     */
    fun getReceivedPayloads(): List<String> = synchronized(receivedPayloads) {
        receivedPayloads.toList()
    }

    /**
     * Clear all received payloads.
     */
    fun clearPayloads() = synchronized(receivedPayloads) {
        receivedPayloads.clear()
    }

    /**
     * Get the base URL for this server.
     */
    fun getBaseUrl(): String = "http://localhost:$port"
}
