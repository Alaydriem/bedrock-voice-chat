<script lang="ts" module>
    import { BootTimeline } from "../js/app/shell/BootTimeline";

    // Module scope, so this runs as the bundle is evaluated rather than when a component
    // mounts. The two marks either side of it split the launch's largest phase into the
    // document arriving, the bundle running, and Svelte hydrating.
    const bodyAt = (window as unknown as { __bvcBodyAt?: number }).__bvcBodyAt;
    if (typeof bodyAt === "number") {
        BootTimeline.shared().markAt("document head + preloader CSS", bodyAt);
    }
    BootTimeline.shared().mark("app bundle evaluated");
</script>

<script lang="ts">
    import { flushSync, onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { warn } from "@charlesportwoodii/tauri-plugin-curia";
    import UpdateOfferDialog from "../components/shell/UpdateOfferDialog.svelte";
    import { UpdateOffer } from "../js/app/shell/UpdateOffer";
    import { SentryManager } from "../js/sentry";
    import Analytics from "../js/app/analytics";
    import { ReactivityProbe } from "../js/app/services/ReactivityProbe.svelte";
    import { ReactivityWatchdog } from "../js/app/services/ReactivityWatchdog";

    interface Props {
        children?: import("svelte").Snippet;
    }
    let { children }: Props = $props();

    // Rooted here so the offer reaches every screen a launch can land on: sign-in, the
    // server list and the dashboard. The fault screens are left alone, and one of them is
    // the install itself.
    const updateOffer = new UpdateOffer();
    let offeredVersion = $state<string | null>(null);
    const offerOpen = $derived(offeredVersion !== null && !page.url.pathname.startsWith("/error"));

    function acceptUpdate(): void {
        const href = updateOffer.accept();
        if (href) void goto(href);
    }

    onMount(() => {
        const unsubscribeOffer = updateOffer.offer.subscribe((v) => (offeredVersion = v));
        void updateOffer.check();

        // Svelte mounts children before their parents, so the root layout's `onMount` is the
        // last one to run. This marks the whole tree being up, not the layout alone.
        BootTimeline.shared().mark("svelte tree mounted");
        SentryManager.initialize();

        // A webview reload is not a launch: the Rust side keeps running, so AppStarted does
        // not fire and every timing after this point is measured from a process that has
        // been up for a while. Rooted here because a reload restores whichever route was
        // open, not the first one.
        //
        // Read from the navigation entry rather than from a flag this app sets, because the
        // reload that matters most is the one nothing in the app asked for.
        const navigation = performance.getEntriesByType?.("navigation")?.[0] as
            | PerformanceNavigationTiming
            | undefined;
        if (navigation?.type === "reload") {
            Analytics.track("Reload", { route: window.location.pathname });
        }

        // A long suspension on Android can wedge Svelte's scheduler: taps land, handlers
        // run, nothing paints. The watchdog probes on every return to visibility and
        // flushes the backlog when the scheduler failed to. Rooted here so every route is
        // covered — the wedge does not care which screen is up.
        const probe = new ReactivityProbe();
        const watchdog = new ReactivityWatchdog(probe, flushSync, (message) => {
            void warn(message);
            SentryManager.warning(message);
        });
        watchdog.start();

        return () => {
            unsubscribeOffer();
            watchdog.cleanup();
            probe.cleanup();
        };
    });
</script>

{@render children?.()}

<UpdateOfferDialog
    open={offerOpen}
    onaccept={acceptUpdate}
    ondismiss={() => updateOffer.dismiss()}
/>
