<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";
    import type { GroupRowView } from "../../js/app/dashboard/GroupRowView";
    import type { ProtectResult } from "../../js/app/managers/ProtectedGroupManager";
    import GroupRow from "./GroupRow.svelte";
    import ProtectedLine from "./ProtectedLine.svelte";

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
        onprotect?: (id: string) => Promise<ProtectResult>;
        /** This client is in a protected group: nearby players cannot hear it. */
        protectedJoined?: boolean;
        /** The code members compare out of band, from the group's current epoch. */
        safetyCode?: string | null;
        /** Why this client last left a protected group, as copy. */
        protectedNotice?: string | null;
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
        onprotect,
        protectedJoined = false,
        safetyCode = null,
        protectedNotice = null,
    }: Props = $props();

    /**
     * Which row's action tray is out, held here rather than in the rows.
     *
     * One open tray at a time, and a row cannot enforce that about its siblings without knowing
     * about them. Owning it at the list level means opening one closes the rest for free.
     */
    let openId = $state<string | null>(null);

    const joinedRow = $derived(groups.find((group) => group.joined) ?? null);
</script>

<!-- One card holding flush rows, rather than a stack of separately-rounded ones. -->
<div class="rad-group-list">
    {#if protectedJoined}
        <ProtectedLine
            {safetyCode}
            groupName={joinedRow?.name ?? ""}
            memberCount={joinedRow?.members.length ?? 0}
        />
    {/if}

    <button class="rad-new-group" onclick={oncreate}>
        <Icon name="plus" />
        {I18n.t("New group")}
    </button>

    {#if protectedNotice}
        <div class="rad-protected-notice" role="alert">{protectedNotice}</div>
    {/if}

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
            {onprotect}
            onopen={(id) => (openId = id)}
        />
    {/each}
</div>
