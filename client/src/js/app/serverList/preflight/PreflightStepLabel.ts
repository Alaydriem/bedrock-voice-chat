import { I18n } from "#lib/i18n/index.js";
import type { PreflightStepName } from "./PreflightStepName";

/** What a preflight check is called on screen. */
export class PreflightStepLabel {
    static of(name: PreflightStepName): string {
        switch (name) {
            case "credentials":
                return I18n.t("Credentials");
            case "handshake":
                return I18n.t("Handshake");
            case "protocol":
                return I18n.t("Protocol");
            case "voicePath":
                return I18n.t("Voice path");
        }
    }
}
