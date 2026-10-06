package com.alaydriem.bedrockvoicechat.admin

interface AdminTarget {
    fun admin(request: AdminRequest): AdminResult

    /** The native last error is thread-local: call this on the thread that called [admin]. */
    fun lastError(): String?
}
