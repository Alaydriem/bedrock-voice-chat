<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import type { GroupRowView } from "../../js/app/dashboard/GroupRowView";
    import GroupRow from "./GroupRow.svelte";

    interface Props {
        groups: readonly GroupRowView[];
        now: number;
        /**
         * A row to open the editor on as soon as it appears.
         *
         * Creating a group opens its editor, and the row does not exist at the moment the create
         * returns — so the id is held here until the channel list catches up and renders it.
         */
        editId?: string | null;
        onjoin: (id: string) => void;
        oncreate: () => void;
        onedit?: (id: string | null) => void;
        onleave?: (id: string) => void;
        onclosegroup?: (id: string) => void;
        onrename?: (id: string, name: string) => void;
    }
    let {
        groups,
        now,
        editId = null,
        onjoin,
        oncreate,
        onedit,
        onleave,
        onclosegroup,
        onrename,
    }: Props = $props();

    /**
     * Which row's action tray is out, held here rather than in the rows.
     *
     * One open tray at a time, and a row cannot enforce that about its siblings without knowing
     * about them. Owning it at the list level means opening one closes the rest for free.
     */
    let openId = $state<string | null>(null);
</script>

<!-- One card holding flush rows, rather than a stack of separately-rounded ones. -->
<div class="rad-group-list">
<button class="rad-new-group" onclick={oncreate}>
    <Icon name="plus" /> New group
</button>

{#each groups as group (group.id)}
    <GroupRow
        {group}
        {now}
        open={openId === group.id}
        {editId}
        {onjoin}
        {onedit}
        {onleave}
        {onclosegroup}
        {onrename}
        onopen={(id) => (openId = id)}
    />
{/each}
</div>
