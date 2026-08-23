<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrock: BedrockManager;
    }
    let { bedrock }: Props = $props();

    let open = $state(false);
    let code = $state("");
    let url = $state("");
    let failure = $state("");
    let copied = $state(false);
    let restoring = $state(false);

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(bedrock.showLoginModal.subscribe((v) => (open = v)));
        unsubs.push(bedrock.deviceCode.subscribe((v) => (code = v)));
        unsubs.push(bedrock.deviceUrl.subscribe((v) => (url = v)));
        unsubs.push(bedrock.loginError.subscribe((v) => (failure = v)));
        unsubs.push(bedrock.codeCopied.subscribe((v) => (copied = v)));
        unsubs.push(bedrock.isRestoringAuth.subscribe((v) => (restoring = v)));
    });

    onDestroy(() => {
        for (const off of unsubs) off();
    });
</script>

<!-- Device-code sign-in. There is no Done button: the poll decides. -->
{#if open}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <!--
      Laid out like the sign-in screen rather than a settings form: a kicker, the code as the
      one thing worth reading, then where to take it. The address was a read-only text input
      inside a two-column row, which nobody types into and which cramped the code beside it.
    -->
    <div class="rad-modal rad-modal--wide is-open">
        <h5 class="rad-modal__title">{I18n.t("Sign in with Microsoft")}</h5>
        <p>
            {I18n.t("Open the page on any device and enter this code. This window updates by itself once you are done.")}
        </p>

        <div style="margin-top: 20px; display: flex; flex-direction: column; gap: 7px">
            <span class="rad-label">{I18n.t("Your code")}</span>
            <span style="display: flex; align-items: center; gap: 12px">
                <span
                    class="rad-kbd"
                    style="font-size: 1.75rem; letter-spacing: 0.1em; padding: 10px 16px"
                >
                    {code || "…"}
                </span>
                <button
                    class="rad-icon-btn"
                    onclick={() => void bedrock.copyDeviceCode()}
                    aria-label={I18n.t("Copy the code")}
                >
                    <Icon name="copy" />
                </button>
            </span>
            <span class="rad-label">
                {copied ? I18n.t("Copied.") : I18n.t("It expires after a few minutes.")}
            </span>
        </div>

        <div style="margin-top: 20px; display: flex; flex-direction: column; gap: 7px">
            <span class="rad-label">{I18n.t("Then go to")}</span>
            <button
                class="rad-link-card"
                onclick={() => void bedrock.openLoginUrl()}
                aria-label={I18n.t("Open the sign-in page")}
            >
                {url || "microsoft.com/link"} <Icon name="ext" />
            </button>
        </div>

        {#if restoring}
            <div class="rad-callout" style="margin-top: 12px">
                <span><StatusChip severity="idle">{I18n.t("Waiting")}</StatusChip> {I18n.t("Watching for your sign-in.")}</span>
            </div>
        {/if}

        {#if failure}
            <div class="rad-callout rad-callout--bad" style="margin-top: 12px">
                <span>{failure}</span>
            </div>
        {/if}

        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => void bedrock.closeLoginModal()}>{I18n.t("Cancel")}</button>
            <button class="rad-btn rad-btn--primary" onclick={() => void bedrock.openLoginUrl()}>
                <Icon name="ext" /> {I18n.t("Open the page")}
            </button>
        </div>
    </div>
{/if}
