<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onDestroy } from "svelte";
    import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { info } from "@tauri-apps/plugin-log";

    /**
     * Backend notifications, as a toast.
     *
     * The kit's rule is that a toast never carries a decision, and these do not: the
     * backend has already acted by the time it says so. A level only changes the border
     * and how long the message stays.
     */
    interface Payload {
        title?: string;
        body?: string;
        level?: string;
    }

    const COOLDOWN_MS = 10000;
    const DURATION_MS = { info: 5000, warn: 5000, bad: 8000 } as const;

    type Severity = keyof typeof DURATION_MS;

    const SEVERITY: Record<string, Severity> = {
        info: "info",
        warn: "warn",
        warning: "warn",
        error: "bad",
    };

    const recent = new Map<string, number>();

    let text = $state("");
    let severity = $state<Severity>("info");
    let shown = $state(false);
    let timer: ReturnType<typeof setTimeout> | null = null;

    let unlisten: (() => void) | null = null;
    let destroyed = false;

    function dismiss(): void {
        shown = false;
        if (timer) clearTimeout(timer);
        timer = null;
    }

    function show(payload: Payload): void {
        const next = `${payload.title ?? ""}: ${payload.body ?? ""}`;
        const now = Date.now();
        // The same message repeating is the backend retrying, not news.
        if (now - (recent.get(next) ?? 0) < COOLDOWN_MS) return;
        recent.set(next, now);

        text = next;
        severity = SEVERITY[payload.level ?? "info"] ?? "info";
        shown = true;

        if (timer) clearTimeout(timer);
        timer = setTimeout(dismiss, DURATION_MS[severity]);
    }

    getCurrentWebviewWindow()
        .listen("notification", (event: { payload?: Payload }) => {
            info(`Notification received: ${JSON.stringify(event.payload)}`);
            show(event.payload ?? {});
        })
        .then((fn) => {
            if (destroyed) {
                fn();
                return;
            }
            unlisten = fn;
        });

    onDestroy(() => {
        destroyed = true;
        unlisten?.();
        if (timer) clearTimeout(timer);
    });
</script>

<div
    class="rad-toast"
    class:is-on={shown}
    class:rad-toast--warn={severity === "warn"}
    class:rad-toast--bad={severity === "bad"}
    role="status"
    aria-live="polite"
>
    {text}
    {#if severity === "bad"}
        <button type="button" class="rad-toast__close" aria-label={I18n.t("Dismiss")} onclick={dismiss}>
            ×
        </button>
    {/if}
</div>
