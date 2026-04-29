<script lang="ts">
    interface Props {
        open: boolean;
        title: string;
        message: string;
        confirmLabel?: string;
        cancelLabel?: string;
        confirmVariant?: 'danger' | 'primary';
        onConfirm: () => void | Promise<void>;
        onCancel: () => void;
    }

    let {
        open,
        title,
        message,
        confirmLabel = "Confirm",
        cancelLabel = "Cancel",
        confirmVariant = "primary",
        onConfirm,
        onCancel,
    }: Props = $props();

    let working = $state(false);

    async function handleConfirm() {
        if (working) return;
        working = true;
        try {
            await onConfirm();
        } finally {
            working = false;
        }
    }

    let confirmClasses = $derived(
        confirmVariant === 'danger'
            ? "bg-error hover:bg-error-focus text-white"
            : "bg-primary hover:bg-primary-focus dark:bg-accent dark:hover:bg-accent-focus text-white"
    );
</script>

{#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
        class="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm"
        onclick={() => { if (!working) onCancel(); }}
    >
        <div
            class="w-[min(420px,90vw)] rounded-2xl bg-white p-6 shadow-2xl dark:bg-navy-700"
            role="dialog"
            aria-modal="true"
            aria-labelledby="confirm-modal-title"
            onclick={(e) => e.stopPropagation()}
        >
            <h3 id="confirm-modal-title" class="text-lg font-semibold text-slate-800 dark:text-navy-100">
                {title}
            </h3>
            <p class="mt-3 text-sm text-slate-600 dark:text-navy-200">
                {message}
            </p>
            <div class="mt-6 flex items-center justify-end gap-2">
                <button
                    class="btn btn-sm font-medium text-slate-700 hover:bg-slate-100
                           dark:text-navy-100 dark:hover:bg-navy-600
                           disabled:opacity-50 disabled:cursor-not-allowed"
                    onclick={onCancel}
                    disabled={working}
                >
                    {cancelLabel}
                </button>
                <button
                    class="btn btn-sm font-medium {confirmClasses}
                           disabled:opacity-50 disabled:cursor-not-allowed"
                    onclick={handleConfirm}
                    disabled={working}
                >
                    {working ? "Working…" : confirmLabel}
                </button>
            </div>
        </div>
    </div>
{/if}
