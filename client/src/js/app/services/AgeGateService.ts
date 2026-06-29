import { ageSignal } from "tauri-plugin-age-signals";
import { warn } from "@tauri-apps/plugin-log";
import type { AgeGateDecision } from "../../bindings/AgeGateDecision";
import type FeatureFlagService from "./FeatureFlagService";

export default class AgeGateService {
    private featureFlags: FeatureFlagService;

    constructor(featureFlags: FeatureFlagService) {
        this.featureFlags = featureFlags;
    }

    async evaluate(minimumAge: number): Promise<AgeGateDecision> {
        if (!(await this.featureFlags.getAgeSignalsEnabled())) {
            return "allow";
        }

        const clamped = Math.min(18, Math.max(13, Math.trunc(minimumAge)));
        try {
            const result = await ageSignal(clamped);
            return result === "belowAgeGate" ? "block" : "allow";
        } catch (e) {
            warn("Age signal check failed, proceeding: " + e);
            return "allow";
        }
    }
}
