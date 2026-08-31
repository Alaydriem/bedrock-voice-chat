package com.alaydriem.bedrockvoicechat.svc

enum class PairingOutcome {
    Paired,
    AlreadyPaired,
    WrongCode,
    Expired,
    Unreachable,
    NotEligible,
    EmbeddedMode,
}

/**
 * One attempt to redeem a pairing code against the external BVC server.
 *
 * `enrol` is injected so the decision this class makes — whether to dial at all, and what
 * the server is told — is testable without the native SDK.
 *
 * `onPaired` opens the bridge the redemption has just made possible. The bridge's connect
 * thread runs once, when Simple Voice Chat hands over its API, and ends there when nothing
 * has granted this node yet; redemption is the event that changes that answer. Which
 * outcomes count as that event is decided here rather than in each platform's command, so
 * Paper and Fabric cannot answer it differently.
 */
class PairingRequest(
    private val eligibility: PeeringEligibility,
    private val onPaired: () -> Unit,
    private val enrol: (peerlink: String, code: String) -> PairingOutcome,
) {

    fun submit(code: String): PairingOutcome {
        val peerlink = eligibility.resolve() ?: return PairingOutcome.NotEligible

        val outcome = enrol(peerlink, normalize(code))
        // A node the server already knows is the answer an operator gets when they run the
        // command a second time, and their reason for running it again is usually that
        // voice is not working. Both outcomes leave a grant in place for this node, which
        // is all the bridge needs to dial.
        if (outcome == PairingOutcome.Paired || outcome == PairingOutcome.AlreadyPaired) {
            onPaired()
        }

        return outcome
    }

    companion object {
        // Mirrors PairingCode::normalize on the server: uppercase, separators removed, and
        // the letters Crockford base32 excludes folded onto the digits they are mistaken
        // for. Normalising on both sides means a mistyped I is a paired bridge rather than
        // a support ticket.
        fun normalize(raw: String): String =
            raw.filter { !it.isWhitespace() && it != '-' }
                .map {
                    when (it.uppercaseChar()) {
                        'I', 'L' -> '1'
                        'O' -> '0'
                        else -> it.uppercaseChar()
                    }
                }
                .joinToString("")
    }
}
