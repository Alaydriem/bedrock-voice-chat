<script lang="ts">
    import { onDestroy } from "svelte";
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";
    import GameNameUtils from "../../js/app/utils/GameNameUtils";
    import type { JoinPhase } from "../../js/app/managers/ProtectedGroupManager";
    import CodeDigits from "./CodeDigits.svelte";

    interface Props {
        name: string;
        phase: JoinPhase;
        code: string | null;
        checkId: string | null;
        /** The canonical identity of the member checking this client. */
        checker: string | null;
        onask: () => void;
        onconfirm: (checkId: string, matches: boolean) => void;
        oncancel: () => void;
    }
    let { name, phase, code, checkId, checker, onask, onconfirm, oncancel }: Props = $props();

    const member = $derived(checker ? GameNameUtils.stripPrefix(checker) : "");
    const ended = $derived(
        phase === "refused" || phase === "mismatch" || phase === "expired" || phase === "no_admitter",
    );

    /** Seconds spent waiting, so a member who has not answered yet does not look like a hang. */
    let elapsed = $state(0);
    let timer: ReturnType<typeof setInterval> | null = null;
    $effect(() => {
        if (phase === "waiting") {
            elapsed = 0;
            timer = setInterval(() => (elapsed += 1), 1_000);
            return () => {
                if (timer) clearInterval(timer);
                timer = null;
            };
        }
    });
    onDestroy(() => {
        if (timer) clearInterval(timer);
    });

    const minutes = $derived(`${Math.floor(elapsed / 60)}:${String(elapsed % 60).padStart(2, "0")}`);

    function onkeydown(event: KeyboardEvent): void {
        if (event.key === "Escape") oncancel();
    }
</script>

<svelte:window {onkeydown} />

<div class="rad-scrim rad-scrim--modal is-on" aria-hidden="true"></div>
<div class="rad-modal rad-modal--dock is-open" role="dialog" aria-modal="true" aria-labelledby="join-protected-title">
    <div class="rad-dialog-head">
        <span class="rad-seal"><Icon name="lock" /></span>
        <h5 class="rad-modal__title" id="join-protected-title">
            {#if phase === "code"}
                {I18n.tf("Read this code to {member}", { member })}
            {:else if phase === "joining"}
                {I18n.tf("Joining {name}", { name })}
            {:else if phase === "refused"}
                {I18n.tf("{member} did not admit you", { member })}
            {:else if phase === "mismatch"}
                {I18n.t("The codes did not match")}
            {:else if phase === "expired"}
                {I18n.t("No member answered")}
            {:else if phase === "no_admitter"}
                {I18n.t("No member is online")}
            {:else}
                {I18n.tf("Join {name}", { name })}
            {/if}
        </h5>
    </div>

    {#if phase === "ask"}
        <p>{I18n.t("A member must admit you.")}</p>
    {:else if phase === "waiting"}
        <div class="rad-wait">
            <span>{I18n.t("Waiting for a member")}</span>
            <span class="rad-wait__elapsed">{minutes}</span>
        </div>
    {:else if phase === "code" && code}
        <CodeDigits {code} />
        <p>{I18n.t("In person, by phone or on Discord. Not in game chat.")}</p>
    {/if}

    {#if phase !== "joining"}
        <div class="rad-modal__actions">
            {#if phase === "ask"}
                <button class="rad-btn" onclick={oncancel}>{I18n.t("Cancel")}</button>
                <button class="rad-btn rad-btn--primary" onclick={onask}>{I18n.t("Ask to join")}</button>
            {:else if phase === "waiting"}
                <button class="rad-btn" onclick={oncancel}>{I18n.t("Cancel")}</button>
            {:else if phase === "code" && checkId}
                <button class="rad-btn" onclick={() => onconfirm(checkId, false)}>{I18n.t("No match")}</button>
                <button class="rad-btn rad-btn--primary" onclick={() => onconfirm(checkId, true)}
                    >{I18n.t("It matches")}</button
                >
            {:else if ended}
                <button class="rad-btn" onclick={oncancel}>{I18n.t("Close")}</button>
                <button class="rad-btn rad-btn--primary" onclick={onask}>{I18n.t("Ask again")}</button>
            {/if}
        </div>
    {/if}
</div>
