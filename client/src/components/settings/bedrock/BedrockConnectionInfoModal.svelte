<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { BedrockConnectionInfo } from "../../../js/bindings/BedrockConnectionInfo";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const connectionInfo = bedrockManager.connectionInfo;

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
        class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
        onclick={dismiss}
    >
        <div
            class="w-[min(520px,92vw)] rounded-2xl bg-white p-6 shadow-2xl dark:bg-navy-700"
            role="dialog"
            aria-modal="true"
            aria-labelledby="bedrock-connection-info-title"
            onclick={(e) => e.stopPropagation()}
        >
            <h3
                id="bedrock-connection-info-title"
                class="text-lg font-semibold text-slate-800 dark:text-navy-100"
            >
                {isRealm(info) ? "Realm" : "Proxy"} is running
            </h3>

            <div class="mt-3 rounded-lg border-l-4 border-warning bg-warning/10 p-3">
                <p class="text-sm font-semibold text-slate-800 dark:text-navy-100">
                    Voice works only between players on this BVC server
                </p>
                <p class="mt-1 text-xs+ leading-relaxed text-slate-600 dark:text-navy-200">
                    This enables proximity voice between players connected to this same BVC server.
                    All players MUST be connected to the same BVC server. If you are not connected to the same server, logout and reconnect to the correct one.
                </p>
            </div>

            <p class="mt-4 text-sm text-slate-600 dark:text-navy-200">
                Open Minecraft, go to <b>Servers</b> → <b>Add Server</b>, and
                enter one of these:
            </p>

            <div class="mt-4 space-y-3">
                <div class="rounded-lg border border-slate-200 dark:border-navy-500 p-3">
                    <div class="text-xs uppercase tracking-wide text-slate-500 dark:text-navy-300">
                        On the same PC (Bedrock for Windows)
                    </div>
                    <div class="mt-1 flex items-center justify-between gap-2">
                        <code class="text-sm text-slate-800 dark:text-navy-100">
                            {info.local_address}:{info.port}
                        </code>
                        <button
                            class="btn btn-xs text-slate-700 hover:bg-slate-100 dark:text-navy-100 dark:hover:bg-navy-600"
                            onclick={() => copyText(`${info.local_address}:${info.port}`, "local")}
                        >
                            {copiedLabel === "local" ? "Copied" : "Copy"}
                        </button>
                    </div>
                </div>

                <div class="rounded-lg border border-slate-200 dark:border-navy-500 p-3">
                    <div class="text-xs uppercase tracking-wide text-slate-500 dark:text-navy-300">
                        From another device on your LAN (phone, tablet)
                    </div>
                    <div class="mt-1 flex items-center justify-between gap-2">
                        <code class="text-sm text-slate-800 dark:text-navy-100">
                            {info.lan_address}:{info.port}
                        </code>
                        <button
                            class="btn btn-xs text-slate-700 hover:bg-slate-100 dark:text-navy-100 dark:hover:bg-navy-600"
                            onclick={() => copyText(`${info.lan_address}:${info.port}`, "lan")}
                        >
                            {copiedLabel === "lan" ? "Copied" : "Copy"}
                        </button>
                    </div>
                </div>

                <div class="rounded-lg border border-slate-200 dark:border-navy-500 p-3">
                    <div class="text-xs uppercase tracking-wide text-slate-500 dark:text-navy-300">
                        Forwarding to
                    </div>
                    <div class="mt-1 text-sm text-slate-800 dark:text-navy-100">
                        {info.remote_label}
                    </div>
                </div>

                <div class="rounded-lg border border-dashed border-slate-300 dark:border-navy-500 p-3">
                    <div class="text-xs uppercase tracking-wide text-slate-500 dark:text-navy-300">
                        If you set your device's DNS to BVC
                    </div>
                    <div class="mt-1 flex items-center justify-between gap-2">
                        <code class="text-sm text-slate-800 dark:text-navy-100">
                            {info.hive_dns_hostname}
                        </code>
                        <button
                            class="btn btn-xs text-slate-700 hover:bg-slate-100 dark:text-navy-100 dark:hover:bg-navy-600"
                            onclick={() => copyText(info.hive_dns_hostname, "dns")}
                        >
                            {copiedLabel === "dns" ? "Copied" : "Copy"}
                        </button>
                    </div>
                    <div class="mt-2 text-xs text-slate-500 dark:text-navy-300">
                        Point Minecraft at the Hive hostname and BVC's DNS service
                        will route it to this proxy. Skip this if you haven't set
                        your device DNS to BVC.
                    </div>
                </div>
            </div>

            <div class="mt-6 flex items-center justify-end gap-2">
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
