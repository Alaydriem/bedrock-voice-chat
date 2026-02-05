<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { listen } from '@tauri-apps/api/event';
    import type { ConnectionHealth } from '../../../js/bindings/ConnectionHealth';

    let health: ConnectionHealth = { status: 'Connected' };
    let unlisten: (() => void) | undefined;

    onMount(async () => {
        unlisten = await listen<ConnectionHealth>('connection_health', (event) => {
            health = event.payload;
        });
    });

    onDestroy(() => {
        if (unlisten) {
            unlisten();
        }
    });

    $: statusColor = {
        'Connected': 'bg-green-500',
        'Reconnecting': 'bg-yellow-500 animate-pulse',
        'Disconnected': 'bg-red-500',
        'Failed': 'bg-red-500',
        'VersionMismatch': 'bg-red-500'
    }[health.status];

    $: tooltip = health.status === 'Reconnecting'
        ? `Reconnecting (attempt ${health.attempt})...`
        : health.status === 'VersionMismatch'
        ? 'Version mismatch - update required'
        : health.status;
</script>

<button
    class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
    data-tooltip={tooltip}
    aria-label="Network Status: {tooltip}"
    title={tooltip}
    disabled
>
    <span class="w-3 h-3 rounded-full {statusColor}"></span>
</button>
