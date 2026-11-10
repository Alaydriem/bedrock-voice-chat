package com.alaydriem.bedrockvoicechat.svc.link

/**
 * An SVC call ran on the server thread and threw. Unlike [SvcUnavailableException] the answer is
 * known: the call failed, and the same call is expected to fail again. The linker skips the one
 * event or group it was working on instead of waiting for it.
 */
class SvcCallFailedException(cause: Throwable?) : Exception("Simple Voice Chat call failed: $cause", cause)
