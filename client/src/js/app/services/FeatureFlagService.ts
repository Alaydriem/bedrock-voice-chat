import { invoke } from "@tauri-apps/api/core";

export default class FeatureFlagService {
    async getAgeSignalsEnabled(): Promise<boolean> {
        return await invoke<boolean>("get_age_signals_enabled");
    }

    async refresh(): Promise<void> {
        await invoke("refresh_feature_flags");
    }
}
