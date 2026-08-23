<script lang="ts">
    import type { Snippet } from "svelte";
    import PlatformDetector from "../../js/app/utils/PlatformDetector";

    interface Props {
        children: Snippet;
    }
    let { children }: Props = $props();

    /**
     * The mobile build. Not the same as a narrow window, which is a container query.
     *
     * The kit decides everything else against `@container rad`, and that is the right
     * default — but a container query cannot tell a tablet from a desktop window of the
     * same width, and a tablet in landscape is wider than every breakpoint the kit has.
     * The rules that need the difference read it from this class.
     *
     * Read synchronously. `plugin-os` returns a value injected at startup, so awaiting it
     * only guarantees a first frame rendered as desktop before the right one replaces it.
     */
    const mobile = new PlatformDetector().mobile();
</script>

<!--
  The kit's fluid modifier fills whatever it is given, which leaves someone to give
  it a height. That is this wrapper: the reference pages hand the frame a stage, and
  in the app the window is the stage.

  Screens are positioned against this element, so it stays the only relative
  ancestor and holds nothing but the grain and whichever screen is current.
-->
<div class="rad-app-stage">
    <div class="rad-frame rad-frame--fluid" class:rad-frame--mobile={mobile}>
        <div class="rad-grain"></div>
        {@render children()}
    </div>
</div>

<style>
    /**
     * Pinned to the viewport rather than sized with `100dvh`.
     *
     * On the Android WebView the two are not the same: the view is inset below the
     * status bar, but `dvh` resolves against the whole window including it. A stage that
     * started below the status bar and was then given the full window's height ran off
     * the bottom of the screen by exactly that inset — and what sits at the bottom of a
     * screen is the footbar, so the primary action of every page was underneath the
     * navigation area. `inset: 0` on a fixed element is the visible viewport by
     * definition, and it is also immune to anything the surrounding document does to
     * the body, which still carries the old theme's classes.
     */
    .rad-app-stage {
        position: fixed;
        inset: 0;
        overflow: hidden;
    }
</style>
