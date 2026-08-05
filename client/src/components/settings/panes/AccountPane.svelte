<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import { AboutManager } from "../../../js/app/managers/settings/AboutManager";
    import { AccountManager } from "../../../js/app/managers/settings/AccountManager";
    import type { DiscordLinkStatus } from "../../../js/bindings/DiscordLinkStatus";

    interface Props {
        onsignout: () => void;
    }
    let { onsignout }: Props = $props();

    const account = new AccountManager();
    // Discord link state lives on the About manager.
    const about = new AboutManager();

    let gamertag = $state("");
    let gamerpic = $state("");
    let javaName = $state<string | null>(null);
    let linking = $state(false);
    let linkError = $state("");
    let desktop = $state(true);
    let discord = $state<DiscordLinkStatus | null>(null);
    let discordBusy = $state(false);
    let discordError = $state("");

    const initials = $derived(gamertag.slice(0, 2).toUpperCase() || "??");

    /** Linking needs the native OAuth window, which is desktop-only. */
    const javaMeta = $derived(
        javaName
            ? `LINKED AS ${javaName.toUpperCase()}`
            : desktop
              ? "NOT LINKED"
              : "LINK THIS FROM THE DESKTOP APP",
    );

    /** An expired link keeps no roles, so it reads differently from an absent one. */
    const discordMeta = $derived(
        discord?.expired && discord.linked
            ? "LINK EXPIRED · LINK AGAIN"
            : discord?.linked
              ? `LINKED · ${discord.role_count} ROLE${discord.role_count === 1 ? "" : "S"}`
              : "NOT LINKED",
    );

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(account.gamertag.subscribe((v) => (gamertag = v)));
        unsubs.push(account.gamerpic.subscribe((v) => (gamerpic = v)));
        unsubs.push(account.minecraftUsername.subscribe((v) => (javaName = v)));
        unsubs.push(account.isLinking.subscribe((v) => (linking = v)));
        unsubs.push(account.linkError.subscribe((v) => (linkError = v)));
        unsubs.push(account.isDesktop.subscribe((v) => (desktop = v)));
        unsubs.push(about.discord.subscribe((v) => (discord = v)));
        unsubs.push(about.discordBusy.subscribe((v) => (discordBusy = v)));
        unsubs.push(about.discordError.subscribe((v) => (discordError = v)));
        void account.initialize();
        void about.initialize();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
    });
</script>

<div class="rad-section">
    <div class="rad-card">
        <div class="rad-card__head">You're signed in as</div>
        <div class="rad-account">
            {#if gamerpic}
                <img class="rad-account__badge" src={gamerpic} alt="" />
            {:else}
                <span class="rad-account__badge" style="background: var(--color-rad-brand)">
                    {initials}
                </span>
            {/if}
            <span class="rad-account__text">
                <span class="rad-account__name">{gamertag || "Signed out"}</span>
                <span class="rad-account__meta">SIGNED IN WITH XBOX LIVE</span>
            </span>
        </div>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">Linked Accounts</div>
        
        <div class="rad-account">
            <span class="rad-account__badge" style="background: #5c8a3c">JA</span>
            <span class="rad-account__text">
                <span class="rad-account__name">Minecraft Java</span>
                <span class="rad-account__meta">{javaMeta}</span>
            </span>
            {#if javaName}
                <button class="rad-btn" disabled>Linked</button>
            {:else if desktop}
                <button
                    class="rad-btn"
                    disabled={linking}
                    onclick={() => void account.handleLinkJavaIdentity()}
                >
                    {linking ? "Linking…" : "Link"}
                </button>
            {/if}
        </div>

        <!-- `configured` is whether this build has Discord credentials compiled in. -->
        {#if discord && discord.configured}
            <div class="rad-account">
                <span class="rad-account__badge" style="background: #5865f2">DC</span>
                <span class="rad-account__text">
                    <span class="rad-account__name">Discord</span>
                    <span class="rad-account__meta">{discordMeta}</span>
                </span>
                <button
                    class="rad-btn"
                    disabled={discordBusy}
                    onclick={() =>
                        void about.discordAction(discord.linked ? "discord_unlink" : "discord_link")}
                >
                    {discord.linked ? "Unlink" : "Link"}
                </button>
            </div>
        {/if}
    </div>

    {#if linkError}
        <div class="rad-callout rad-callout--bad"><span>{linkError}</span></div>
    {/if}

    {#if discordError && discord && discord.configured}
        <div class="rad-callout rad-callout--bad"><span>{discordError}</span></div>
    {/if}

    <div class="rad-card">
        <SettingRow
            label="Sign out of this server"
            note="Your other servers stay signed in."
        >
            {#snippet control()}
                <button class="rad-btn rad-btn--danger" onclick={onsignout}>Sign out</button>
            {/snippet}
        </SettingRow>
    </div>
</div>
