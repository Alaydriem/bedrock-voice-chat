package com.alaydriem.bedrockvoicechat.svc

import kotlinx.coroutines.runBlocking
import uniffi.bvc_relay_sdk.SdkEnrolOutcome
import uniffi.bvc_relay_sdk.bvcEnrol
import java.io.File

/**
 * Builds the `PairingRequest` behind `/bvc peer`, for a server running an external BVC.
 *
 * Kept out of the platform modules because Paper and Fabric would otherwise carry the same
 * translation from an SDK outcome to an operator-facing one, and a divergence between them
 * is two different answers to the same command.
 */
object SvcPairing {

    /**
     * `null` in embedded mode, where the mod grants itself and no code exists to redeem,
     * and on a server with no external BVC configured.
     *
     * The eligibility is passed in rather than built here so the command and the bridge's
     * own peerlink lookup share one answer, and one fetch of `/api/config`.
     */
    fun forExternal(
        eligibility: PeeringEligibility?,
        nodeDir: File,
        worlds: () -> List<String>,
        onPaired: () -> Unit,
    ): PairingRequest? {
        if (eligibility == null) {
            return null
        }

        return PairingRequest(eligibility, onPaired) { peerlink, code ->
            // The command handler is on the server thread and the operator is waiting on
            // the answer, so this blocks rather than detaching. `Enrolment` bounds its own
            // dial, which is what keeps that finite.
            val outcome = runBlocking {
                bvcEnrol(nodeDir.absolutePath, peerlink, worlds(), code)
            }

            translate(outcome)
        }
    }

    private fun translate(outcome: SdkEnrolOutcome): PairingOutcome = when (outcome) {
        is SdkEnrolOutcome.Paired -> PairingOutcome.Paired
        is SdkEnrolOutcome.WrongCode -> PairingOutcome.WrongCode
        is SdkEnrolOutcome.Expired -> PairingOutcome.Expired
        // The server knows this node already, which is what an operator running the
        // command twice sees.
        is SdkEnrolOutcome.NotAuthorized -> PairingOutcome.AlreadyPaired
        is SdkEnrolOutcome.NoSharedWorld -> PairingOutcome.NotEligible
        is SdkEnrolOutcome.NoCommonVersion -> PairingOutcome.NotEligible
        is SdkEnrolOutcome.Unreachable -> PairingOutcome.Unreachable
    }
}
