package com.alaydriem.bedrockvoicechat.svc.link

/**
 * The server thread did not answer an SVC call in time, or refused it while stopping. The answer
 * is unknown, not empty, so the linker abandons the pass instead of acting on it.
 */
class SvcUnavailableException : Exception("Simple Voice Chat did not answer in time")
