<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";
    import { PlayerHue } from "$radial/core/sources/PlayerHue";
    import GameNameUtils from "../../js/app/utils/GameNameUtils";
    import type { AdmissionCardState } from "../../js/app/dashboard/AdmissionStackView";
    import CodeDigits from "./CodeDigits.svelte";

    interface Props {
        /** The identity the server gave for the joiner's socket. */
        joinerName: string;
        groupName: string;
        phase: AdmissionCardState["phase"];
        /** Open on its code. At most one card is. */
        expanded: boolean;
        code: string | null;
        checkId: string | null;
        otherMember: string | null;
        secondsLeft: number | null;
        /** Fewer than three checks are open. */
        canCheck: boolean;
        pending?: boolean;
        deferred?: boolean;
        oncheck: () => void;
        ondecide: (checkId: string, allow: boolean) => void;
        onshow: () => void;
        onlater: () => void;
        ondismiss: () => void;
    }
    let {
        joinerName,
        groupName,
        phase,
        expanded,
        code,
        checkId,
        otherMember,
        secondsLeft,
        canCheck,
        pending = false,
        deferred = false,
        oncheck,
        ondecide,
        onshow,
        onlater,
        ondismiss,
    }: Props = $props();

    const joiner = $derived(GameNameUtils.stripPrefix(joinerName));
    const checker = $derived(otherMember ? GameNameUtils.stripPrefix(otherMember) : "");
    const initials = $derived(joiner.slice(0, 2).toUpperCase());
    const hue = $derived(PlayerHue.of(joinerName.toLowerCase()));
    const time = $derived(
        secondsLeft === null
            ? ""
            : `${Math.floor(secondsLeft / 60)}:${String(secondsLeft % 60).padStart(2, "0")}`,
    );
</script>

<div
    class="rad-admit-card"
    class:is-checking={phase === "code" && expanded}
    class:is-quiet={phase === "checking_elsewhere"}
    class:is-fault={phase === "failed"}
    class:is-done={phase === "admitted"}
>
    <div class="rad-admit-card__top">
        <span class="rad-admit-card__face" style="--face: {hue}">{initials}</span>
        <span class="rad-admit-card__name">
            {#if phase === "request"}
                {I18n.tf("Admit {joiner} to {group}?", { joiner, group: groupName })}
            {:else if phase === "code" && expanded}
                {I18n.tf("Check {joiner}'s code", { joiner })}
            {:else if phase === "code"}
                {I18n.tf("{joiner}'s code is ready", { joiner })}
            {:else if phase === "checking_elsewhere"}
                {I18n.tf("{member} is checking {joiner}", { member: checker, joiner })}
            {:else if phase === "admitted"}
                {I18n.tf("{joiner} joined {group}", { joiner, group: groupName })}
            {:else}
                {I18n.t("Check failed")}
            {/if}
        </span>
        {#if phase === "admitted"}
            <Icon name="check" />
        {:else if time && phase !== "failed" && phase !== "checking_elsewhere"}
            <span class="rad-admit-card__time">{time}</span>
        {/if}
    </div>

    {#if phase === "request" && !deferred}
        <div class="rad-admit-card__actions">
            <button class="rad-btn rad-btn--quiet" onclick={onlater}>{I18n.t("Not now")}</button>
            <button class="rad-btn rad-btn--primary" onclick={oncheck} disabled={!canCheck || pending}
                >{I18n.t("Check code")}</button
            >
        </div>
    {:else if phase === "request" && deferred}
        <div class="rad-admit-card__actions rad-admit-card__actions--one">
            <button class="rad-btn rad-btn--quiet" onclick={oncheck} disabled={!canCheck || pending}
                >{I18n.t("Check code")}</button
            >
        </div>
    {:else if phase === "code" && expanded && code && checkId}
        <p class="rad-admit-card__copy">
            {I18n.tf("{joiner} reads you this code. Admit only if it is {joiner} and the code matches.", {
                joiner,
            })}
        </p>
        <CodeDigits {code} />
        <div class="rad-admit-card__actions">
            <button class="rad-btn rad-btn--danger" onclick={() => ondecide(checkId, false)}
                >{I18n.t("Deny")}</button
            >
            <button class="rad-btn rad-btn--primary" onclick={() => ondecide(checkId, true)}
                >{I18n.tf("Admit {joiner}", { joiner })}</button
            >
        </div>
    {:else if phase === "code"}
        <div class="rad-admit-card__actions rad-admit-card__actions--one">
            <button class="rad-btn" onclick={onshow}>{I18n.t("Show code")}</button>
        </div>
    {:else if phase === "failed"}
        <p class="rad-admit-card__copy">{I18n.t("Someone may be interfering.")}</p>
        <div class="rad-admit-card__actions rad-admit-card__actions--one">
            <button class="rad-btn rad-btn--quiet" onclick={ondismiss}>{I18n.t("Dismiss")}</button>
        </div>
    {/if}
</div>
