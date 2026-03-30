<script lang="ts">
    import { onDestroy } from "svelte";
    import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { info } from "@tauri-apps/plugin-log";
    import notification from "../../js/magics/notification";

    const TOAST_COOLDOWN_MS = 10000;
    const recentToasts = new Map<string, number>();

    const levelToVariant: Record<string, string> = {
        'info': 'info',
        'warn': 'warning',
        'warning': 'warning',
        'error': 'error',
    };

    let unlisten: (() => void) | null = null;
    let destroyed = false;

    const appWebview = getCurrentWebviewWindow();
    appWebview.listen('notification', (event: { payload?: { title?: string, body?: string, level?: string } }) => {
        info(`Notification received: ${JSON.stringify(event.payload)}`);
        const variant = levelToVariant[event.payload?.level || 'info'] || 'info';
        const text = `${event.payload?.title || ''}: ${event.payload?.body || ''}`;

        const now = Date.now();
        const lastShown = recentToasts.get(text) || 0;
        if (now - lastShown < TOAST_COOLDOWN_MS) return;
        recentToasts.set(text, now);

        notification({
            text,
            variant,
            duration: variant === 'error' ? 8000 : 5000,
            position: 'right-top',
            hasCloseBtn: true,
        });
    }).then(fn => {
        if (destroyed) { fn(); return; }
        unlisten = fn;
    });

    onDestroy(() => {
        destroyed = true;
        if (unlisten) unlisten();
    });
</script>
