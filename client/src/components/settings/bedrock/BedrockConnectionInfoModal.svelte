<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { BedrockConnectionInfo } from "../../../js/bindings/BedrockConnectionInfo";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const connectionInfo = bedrockManager.connectionInfo;

    // Lock background scroll while the modal is open so touch/scroll doesn't
    // chain through to the settings page behind the overlay.
    $effect(() => {
        if (!$connectionInfo) return;
        const previous = document.body.style.overflow;
        document.body.style.overflow = "hidden";
        return () => {
            document.body.style.overflow = previous;
        };
    });

    let copiedLabel = $state<string | null>(null);
    let copiedTimer: ReturnType<typeof setTimeout> | null = null;

    async function copyText(value: string, label: string): Promise<void> {
        try {
            await navigator.clipboard.writeText(value);
            copiedLabel = label;
            if (copiedTimer) {
                clearTimeout(copiedTimer);
            }
            copiedTimer = setTimeout(() => {
                copiedLabel = null;
                copiedTimer = null;
            }, 1500);
        } catch {
        }
    }

    function dismiss(): void {
        if (copiedTimer) {
            clearTimeout(copiedTimer);
            copiedTimer = null;
        }
        copiedLabel = null;
        bedrockManager.dismissConnectionInfo();
    }

    function isRealm(info: BedrockConnectionInfo): boolean {
        return info.backend === "Realm";
    }
</script>

{#if $connectionInfo}
    {@const info = $connectionInfo}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
        class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 p-0 backdrop-blur-sm sm:p-4"
        onclick={dismiss}
    >
        <div
            class="flex h-full max-h-full w-full flex-col overflow-hidden rounded-none bg-white shadow-2xl dark:bg-navy-700 sm:h-auto sm:max-h-[90vh] sm:w-[min(760px,95vw)] sm:rounded-2xl"
            role="dialog"
            aria-modal="true"
            aria-labelledby="bedrock-connection-info-title"
            onclick={(e) => e.stopPropagation()}
        >
            <div class="shrink-0 border-b border-slate-150 px-6 py-4 dark:border-navy-600">
                <h3
                    id="bedrock-connection-info-title"
                    class="text-lg font-semibold text-slate-800 dark:text-navy-100"
                >
                    {isRealm(info) ? "Realm" : "Proxy"} is running
                </h3>
            </div>

            <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-6 py-4">
                <div class="rounded-lg border-l-4 border-warning bg-warning/10 p-3">
                    <p class="text-sm font-semibold text-slate-800 dark:text-navy-100">
                        Voice works only between players on this BVC server
                    </p>
                    <p class="mt-1 text-xs+ leading-relaxed text-slate-600 dark:text-navy-200">
                        This enables proximity voice between players connected to this same BVC server.
                        All players MUST be connected to the same BVC server. If you are not connected to the same server, logout and reconnect to the correct one.
                    </p>
                </div>

                <p class="mt-4 text-sm font-semibold text-slate-800 dark:text-navy-100">
                    Forwarding to {info.remote_label}
                </p>

                <p class="mt-4 text-sm text-slate-600 dark:text-navy-200">
                    Open Minecraft, go to <b>Servers</b> → <b>Add Server</b>, and
                    enter one of these:
                </p>

                <div class="mt-4 grid grid-cols-1 items-start gap-3 sm:grid-cols-2">
                    {@render copyCard(
                        "On the same PC (Bedrock for Windows)",
                        `${info.local_address}:${info.port}`,
                        "local"
                    )}

                    {@render copyCard(
                        "From another device on your LAN (phone, tablet)",
                        `${info.lan_address}:${info.port}`,
                        "lan"
                    )}

                    {#if info.server_transfer_relay}
                        {@render copyCard(
                            "Through the BVC server (transfer relay)",
                            info.server_transfer_relay,
                            "relay",
                            "Connect here to reach this BVC server's transfer relay, which routes you to your active voice session."
                        )}
                    {/if}

                    {#if info.server_dns_enabled}
                        {@render copyCard(
                            "If you set your device's DNS to BVC",
                            info.hive_dns_hostname,
                            "dns",
                            "Point Minecraft at the Hive hostname and BVC's DNS service will route it to this proxy. Skip this if you haven't set your device DNS to BVC.",
                            true
                        )}
                    {/if}
                </div>
            </div>

            <div class="flex shrink-0 items-center justify-end gap-2 border-t border-slate-150 px-6 py-4 dark:border-navy-600">
                <button
                    class="btn btn-sm font-medium bg-primary hover:bg-primary-focus
                           dark:bg-accent dark:hover:bg-accent-focus text-white"
                    onclick={dismiss}
                >
                    OK
                </button>
            </div>
        </div>
    </div>
{/if}

{#snippet copyCard(title: string, value: string, label: string, note?: string, dashed?: boolean)}
    <div class="rounded-lg border {dashed ? 'border-dashed border-slate-300' : 'border-slate-200'} p-3 dark:border-navy-500">
        <div class="text-xs uppercase tracking-wide text-slate-500 dark:text-navy-300">
            {title}
        </div>
        <div class="mt-1 flex items-center justify-between gap-2">
            <code class="min-w-0 break-all text-sm text-slate-800 dark:text-navy-100">
                {value}
            </code>
            <button
                class="btn btn-xs shrink-0 text-slate-700 hover:bg-slate-100 dark:text-navy-100 dark:hover:bg-navy-600"
                onclick={() => copyText(value, label)}
            >
                {copiedLabel === label ? "Copied" : "Copy"}
            </button>
        </div>
        {#if note}
            <div class="mt-2 text-xs text-slate-500 dark:text-navy-300">
                {note}
            </div>
        {/if}
    </div>
{/snippet}
