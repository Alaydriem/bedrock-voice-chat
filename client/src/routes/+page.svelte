<script lang="ts">
    import "../css/app.css";
    import { onMount } from "svelte";
    import Loader from "$radial/components/Loader.svelte";
    import RadFrame from "../components/shell/RadFrame.svelte";
    import RadScreen from "../components/shell/RadScreen.svelte";
    import Splash from "../js/app/splash.ts";

    /**
     * The launch check runs before anything else, so this is the first thing anyone
     * sees — and it has to be legible immediately. The boot sequence holds gain at
     * zero for its first 1.6s, so it opens on the steady dance instead: an update
     * check is a wait, not an arrival.
     */
    const PHRASES: readonly string[] = [
        "Checking for updates…",
        "Looking for your servers…",
        "Almost there…",
    ];

    onMount(async () => {
        window.App = new Splash();
        await window.App.initialize();
        window.dispatchEvent(new CustomEvent("app:mounted"));
    });
</script>

<RadFrame>
    <RadScreen label="Starting up">
        <div class="rad-launch">
            <Loader loading={true} phrases={PHRASES} slowAfterSeconds={4} />
        </div>

        {#snippet footbar()}
            <span class="rad-label">Bedrock Voice Chat</span>
        {/snippet}
    </RadScreen>
</RadFrame>

<style>
    /* The loader is the whole screen here, so it gets centred rather than put in a
       pane beside copy. */
    .rad-launch {
        flex: 1 1 auto;
        min-height: 0;
        display: flex;
        align-items: center;
        justify-content: center;
    }
</style>
