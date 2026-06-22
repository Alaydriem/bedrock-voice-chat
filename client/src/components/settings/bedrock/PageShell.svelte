<script lang="ts">
    import { onMount, type Snippet } from "svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import TopBar from "./TopBar.svelte";
    import NetworkingCard from "./NetworkingCard.svelte";
    import LogPanel from "./LogPanel.svelte";
    import RestoringAuthBanner from "./RestoringAuthBanner.svelte";
    import SignInCard from "./SignInCard.svelte";
    import StatusMessageCard from "./StatusMessageCard.svelte";
    import ErrorBanner from "./ErrorBanner.svelte";
    import XboxLoginModal from "./XboxLoginModal.svelte";
    import BedrockConnectionInfoModal from "./BedrockConnectionInfoModal.svelte";
    import GatingModal from "./GatingModal.svelte";

    interface Props {
        bedrockManager: BedrockManager;
        title: string;
        signedOutDescription: string;
        showListenPort?: boolean;
        extraActions?: Snippet;
        body: Snippet;
    }

    let {
        bedrockManager,
        title,
        signedOutDescription,
        showListenPort = true,
        extraActions,
        body,
    }: Props = $props();

    const isAuthenticated = bedrockManager.isAuthenticated;
    const isRestoringAuth = bedrockManager.isRestoringAuth;
    const showLoginModal = bedrockManager.showLoginModal;

    onMount(() => { bedrockManager.initialize(); });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    {#if $isRestoringAuth}
        <RestoringAuthBanner />
    {:else if !$isAuthenticated}
        <SignInCard {bedrockManager} description={signedOutDescription} />
    {:else}
        <ErrorBanner {bedrockManager} />
        <TopBar {bedrockManager} {title} {extraActions} />
        <NetworkingCard {bedrockManager} {showListenPort} />
        {@render body()}
        <StatusMessageCard {bedrockManager} />
        <LogPanel {bedrockManager} />
    {/if}
</div>

{#if $showLoginModal}
    <XboxLoginModal {bedrockManager} />
{/if}

<BedrockConnectionInfoModal {bedrockManager} />

<GatingModal {bedrockManager} />
