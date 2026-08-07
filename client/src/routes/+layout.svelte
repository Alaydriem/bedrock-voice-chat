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
    import { onMount } from "svelte";
    import { SentryManager } from "../js/sentry";

    interface Props {
        children?: import("svelte").Snippet;
    }
    let { children }: Props = $props();

    onMount(() => {
        BootTimeline.shared().mark("root layout hydrated");
        SentryManager.initialize();
    });
</script>

{@render children?.()}
