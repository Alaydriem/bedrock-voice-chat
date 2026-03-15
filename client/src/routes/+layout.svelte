<script lang="ts">
    import { onMount } from "svelte";
    import { SentryManager } from "../js/sentry";

    interface Props {
        children?: import("svelte").Snippet;
    }
    let { children }: Props = $props();

    onMount(() => {
        SentryManager.initialize();
    });
</script>

{@render children?.()}
