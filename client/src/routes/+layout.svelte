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
    import { warn } from "@charlesportwoodii/tauri-plugin-curia";
    import { SentryManager } from "../js/sentry";
    import { ReactivityProbe } from "../js/app/services/ReactivityProbe.svelte";
    import { ReactivityWatchdog } from "../js/app/services/ReactivityWatchdog";

    interface Props {
        children?: import("svelte").Snippet;
    }
    let { children }: Props = $props();

    onMount(() => {
        // Svelte mounts children before their parents, so the root layout's `onMount` is the
        // last one to run. This marks the whole tree being up, not the layout alone.
        BootTimeline.shared().mark("svelte tree mounted");
        SentryManager.initialize();

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
            watchdog.cleanup();
            probe.cleanup();
        };
    });
</script>

{@render children?.()}
