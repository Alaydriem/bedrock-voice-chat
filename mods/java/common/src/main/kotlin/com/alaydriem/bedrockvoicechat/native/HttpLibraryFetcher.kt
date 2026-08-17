package com.alaydriem.bedrockvoicechat.native

import java.io.IOException
import java.net.ConnectException
import java.net.URI
import java.net.UnknownHostException
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.net.http.HttpTimeoutException
import java.time.Duration
import javax.net.ssl.SSLException

/**
 * Fetches release assets over HTTPS, classifying every failure into the closed set
 * of outcome values the capability check reports.
 */
class HttpLibraryFetcher : LibraryFetcher {

    private val client: HttpClient = HttpClient.newBuilder()
        .connectTimeout(CONNECT_TIMEOUT)
        // Redirects are resolved by hand rather than followed, so the HTTPS check
        // below applies to the destination as well as the original URL.
        .followRedirects(HttpClient.Redirect.NEVER)
        .build()

    override fun fetch(url: String): ByteArray = fetch(url, 0)

    private fun fetch(url: String, depth: Int): ByteArray {
        if (!url.startsWith("https://")) {
            throw NativeLibraryError.Fetch("tls", "Refusing a non-HTTPS URL: $url")
        }

        if (depth > MAX_REDIRECTS) {
            throw NativeLibraryError.Fetch("io", "Too many redirects fetching $url")
        }

        val request = HttpRequest.newBuilder()
            .uri(URI.create(url))
            .timeout(REQUEST_TIMEOUT)
            .GET()
            .build()

        val response: HttpResponse<ByteArray> = try {
            client.send(request, HttpResponse.BodyHandlers.ofByteArray())
        } catch (e: UnknownHostException) {
            throw NativeLibraryError.Fetch("dns", "Cannot resolve the host for $url", e)
        } catch (e: HttpTimeoutException) {
            throw NativeLibraryError.Fetch("timeout", "Timed out fetching $url", e)
        } catch (e: ConnectException) {
            throw NativeLibraryError.Fetch("refused", "Connection refused for $url", e)
        } catch (e: SSLException) {
            throw NativeLibraryError.Fetch("tls", "TLS failed for $url", e)
        } catch (e: IOException) {
            throw NativeLibraryError.Fetch(classify(e), "Failed to fetch $url", e)
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            throw NativeLibraryError.Fetch("io", "Interrupted fetching $url", e)
        }

        if (response.statusCode() in 300..399) {
            val location = response.headers().firstValue("location").orElse(null)
                ?: throw NativeLibraryError.Fetch(
                    "http_${response.statusCode()}",
                    "Redirect without a Location header for $url"
                )
            return fetch(location, depth + 1)
        }

        if (response.statusCode() != 200) {
            throw NativeLibraryError.Fetch(
                "http_${response.statusCode()}",
                "Unexpected status ${response.statusCode()} for $url"
            )
        }

        return response.body()
    }

    /**
     * The JDK client wraps connect-time failures in a plain IOException, so the
     * cause chain is what distinguishes a name that did not resolve from a host
     * that refused.
     */
    private fun classify(e: IOException): String {
        var cause: Throwable? = e
        while (cause != null) {
            when (cause) {
                is UnknownHostException -> return "dns"
                is ConnectException -> return "refused"
                is SSLException -> return "tls"
                is HttpTimeoutException -> return "timeout"
            }
            cause = cause.cause
        }
        return "io"
    }

    companion object {
        private val CONNECT_TIMEOUT: Duration = Duration.ofSeconds(10)
        private val REQUEST_TIMEOUT: Duration = Duration.ofSeconds(60)
        private const val MAX_REDIRECTS: Int = 5
    }
}
