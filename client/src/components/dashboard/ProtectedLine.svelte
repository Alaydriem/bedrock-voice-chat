<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";
    import SafetyCodeDialog from "./SafetyCodeDialog.svelte";

    interface Props {
        safetyCode: string | null;
        groupName: string;
        memberCount: number;
    }
    let { safetyCode, groupName, memberCount }: Props = $props();

    let open = $state(false);
</script>

<!-- One line at the head of the group list, in place of a banner and an always-on code. -->
<div class="rad-sealed-line" role="status">
    <Icon name="lock" />
    <span>{I18n.t("Players nearby cannot hear you.")}</span>
    <button
        class="rad-sealed-line__code"
        aria-expanded={open}
        disabled={!safetyCode}
        onclick={() => (open = true)}
    >
        <Icon name="shield" />
        {I18n.t("Safety code")}
    </button>
</div>

<SafetyCodeDialog {open} code={safetyCode} {groupName} {memberCount} onclose={() => (open = false)} />
