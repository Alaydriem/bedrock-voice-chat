<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { ProxyServerEntry } from "../../../js/app/managers/bedrock/ProxyServerEntry";

    interface Props {
        bedrockManager: BedrockManager;
        initial?: ProxyServerEntry;
        onClose: () => void;
    }

    let { bedrockManager, initial, onClose }: Props = $props();

    let name = $state(initial?.name ?? "");
    let host = $state(initial?.host ?? "");
    let port = $state(initial?.port ?? 19132);
    let saving = $state(false);
    let error = $state("");

    let isValid = $derived(
        name.trim().length > 0 &&
        host.trim().length > 0 &&
        Number.isInteger(port) && port >= 1 && port <= 65535
    );

    async function save() {
        if (!isValid || saving) {
            return;
        }
        saving = true;
        error = "";
        try {
            if (initial) {
                await bedrockManager.updateProxyServer(initial.id, { name, host, port });
            } else {
                await bedrockManager.addProxyServer(name, host, port);
            }
            onClose();
        } catch (e) {
            error = String(e);
            saving = false;
        }
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4"
    onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
>
    <div class="card w-full max-w-md rounded-2xl p-6 shadow-xl bg-white dark:bg-navy-700 mx-4">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-medium text-slate-800 dark:text-navy-100">
                {initial ? "Edit Server" : "Add Server"}
            </h3>
            <button
                class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 dark:hover:bg-navy-300/20"
                onclick={onClose}
                aria-label="Close"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                </svg>
            </button>
        </div>

        <div class="space-y-4">
            <label class="block">
                <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Name</span>
                <input
                    type="text"
                    class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                           dark:hover:border-navy-400 dark:focus:border-accent"
                    bind:value={name}
                    placeholder="My Server"
                />
            </label>

            <label class="block">
                <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Host</span>
                <input
                    type="text"
                    class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                           dark:hover:border-navy-400 dark:focus:border-accent"
                    bind:value={host}
                    placeholder="play.example.com"
                />
            </label>

            <label class="block">
                <span class="text-xs+ font-medium text-slate-700 dark:text-navy-100">Port</span>
                <input
                    type="number"
                    class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2
                           hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700
                           dark:hover:border-navy-400 dark:focus:border-accent"
                    bind:value={port}
                    min="1"
                    max="65535"
                />
            </label>

            {#if error}
                <div class="rounded-lg bg-error/10 p-3">
                    <p class="text-sm text-error">{error}</p>
                </div>
            {/if}
        </div>

        <div class="mt-6 flex items-center justify-end gap-2">
            <button
                class="btn font-medium text-slate-700 hover:bg-slate-150 dark:text-navy-100 dark:hover:bg-navy-500"
                onclick={onClose}
                disabled={saving}
            >
                Cancel
            </button>
            <button
                class="btn font-medium text-white bg-primary hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus
                       disabled:bg-slate-300 disabled:text-slate-500 dark:disabled:bg-navy-500 dark:disabled:text-navy-300"
                onclick={save}
                disabled={!isValid || saving}
            >
                {saving ? "Saving..." : "Save"}
            </button>
        </div>
    </div>
</div>
