<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import type { SettingsGroup } from "../../js/app/settings/SettingsPane";

    interface Props {
        groups: readonly SettingsGroup[];
        current: string;
        /** The one badge in this list, and it means an update is waiting. */
        badged?: string | null;
        /** The phone's first level, where a row leads somewhere rather than selecting. */
        layout?: "sidebar" | "list";
        onpick: (id: string) => void;
    }
    let { groups, current, badged = null, layout = "sidebar", onpick }: Props = $props();
</script>

{#each groups as group (group.name)}
    {#if group.name}
        <div class={layout === "list" ? "rad-mobile-group" : "rad-nav-group"}>{group.name}</div>
    {/if}
    {#each group.panes as pane (pane.id)}
        {#if layout === "list"}
            <button class="rad-mobile-row" onclick={() => onpick(pane.id)}>
                {pane.title}
                {#if badged === pane.id}
                    <span class="rad-mobile-row__badge">1</span>
                {/if}
                <span class="rad-mobile-row__chevron"><Icon name="chev" /></span>
            </button>
        {:else}
            <button
                class="rad-nav-item"
                class:is-on={pane.id === current}
                aria-current={pane.id === current ? "page" : "false"}
                onclick={() => onpick(pane.id)}
            >
                {pane.title}
                {#if badged === pane.id}
                    <span class="rad-nav-item__badge">1</span>
                {/if}
            </button>
        {/if}
    {/each}
{/each}
