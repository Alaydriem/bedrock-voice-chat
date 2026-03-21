import { invoke } from "@tauri-apps/api/core";
import type { AnalyticsEvent } from "../bindings/AnalyticsEvent";

export default class Analytics {
    static track(event: AnalyticsEvent, data?: Record<string, string | number>): void {
        invoke("track_event", { event, data: data ?? null }).catch(() => {});
    }
}
