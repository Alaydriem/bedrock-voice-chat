<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import { KeybindsManager } from "../../../js/app/managers/settings/KeybindsManager";
    import type { KeybindConfig } from "../../../js/bindings/KeybindConfig";

    const keybinds = new KeybindsManager();

    let config = $state<KeybindConfig | null>(null);
    let editing = $state<keyof KeybindConfig | null>(null);
    let captured = $state("");
    let conflict = $state("");

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(keybinds.config.subscribe((v) => (config = v)));
        unsubs.push(keybinds.editingId.subscribe((v) => (editing = v)));
        unsubs.push(keybinds.capturedCombo.subscribe((v) => (captured = v)));
        unsubs.push(keybinds.conflictError.subscribe((v) => (conflict = v)));
        void keybinds.initialize();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
        keybinds.destroy();
    });

    // What the cap shows: the combination being pressed while listening, the bound one
    // otherwise, and "Not set" for a binding that has been cleared. An empty cap reads
    // as a rendering fault rather than as a deliberate absence.
    function capFor(id: keyof KeybindConfig): string {
        if (editing === id) return captured || "Press a combination";
        const bound = config?.[id];
        return typeof bound === "string" && bound ? keybinds.displayCombo(bound) : "Not set";
    }
</script>

<div class="rad-section">
    <div class="rad-section__note">
        Global shortcuts. They work while Minecraft has focus, which is the entire point of them.
    </div>

    <div class="rad-card">
        {#each keybinds.rows as row (row.id)}
            <SettingRow label={row.label}>
                {#snippet control()}
                    <button
                        class="rad-keycap"
                        class:is-listening={editing === row.id}
                        onclick={() =>
                            editing === row.id
                                ? keybinds.cancelEditing()
                                : keybinds.startEditing(row.id)}
                    >
                        {capFor(row.id)}
                    </button>
                {/snippet}
            </SettingRow>
        {/each}
    </div>

    {#if conflict}
        <div class="rad-callout rad-callout--bad"><span>{conflict}</span></div>
    {/if}

    <div class="rad-callout">
        <span>Click a binding, then press the combination. <b>Escape cancels, Delete clears it.</b></span>
    </div>

    <div class="rad-card">
        <SettingRow
            label="Back to the defaults"
            note="Restores every shortcut above, not just the one you were editing."
        >
            {#snippet control()}
                <button class="rad-btn" onclick={() => keybinds.resetAll()}>Reset all</button>
            {/snippet}
        </SettingRow>
    </div>
</div>
