<script lang="ts">
    function goToSubscriptions() {
        window.dispatchEvent(new CustomEvent("settings-navigate", { detail: "subscriptions.svelte" }));
    }
</script>

<div class="card p-4 lg:p-6 border-l-4 border-warning">
    <p class="text-sm text-slate-700 dark:text-navy-200">
        Realms Connect requires an active subscription, a free-weekend window, or a membership code.
        Patreon and YouTube members receive a code as a benefit.
    </p>
    <button
        class="btn mt-3 bg-primary font-medium text-white hover:bg-primary-focus"
        onclick={goToSubscriptions}
    >
        View subscription options
    </button>
</div>
