<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Ring from "$radial/components/Ring.svelte";
    import { Handoff, type Point } from "$radial/core/controllers/Handoff";
    import type { NearbyPlayer } from "../../js/app/dashboard/NearbyPlayer";
    import { RingCast } from "../../js/app/dashboard/RingCast";

    interface Props {
        /** Players the feed can see but who are not close enough to hear. */
        approaching: readonly NearbyPlayer[];
        /** How far the feed reaches, for the falloff the marks are drawn with. */
        scope: number;
        /** Fades out once anybody is in earshot and the roster takes over. */
        gone?: boolean;
        /** False when the link is down. Outranks everything else this can show. */
        connected?: boolean;
        /** Set while a reconnect is in flight, which is a different sentence to say. */
        reconnecting?: boolean;
        /** Opens the status panel. This screen is where a broken link is first noticed. */
        onstatus?: () => void;
    }
    let {
        approaching,
        scope,
        gone = false,
        connected = true,
        reconnecting = false,
        onstatus,
    }: Props = $props();

    const cast = $derived(RingCast.of(approaching, scope, connected));

    const kicker = $derived(
        !connected
            ? reconnecting
                ? "Reconnecting"
                : "Disconnected"
            : approaching.length
              ? "Approaching"
              : "Listening",
    );

    const headline = $derived(
        !connected
            ? reconnecting
                ? "Trying to reach the server"
                : "Not connected"
            : approaching.length === 0
              ? "Nobody nearby"
              : approaching.length === 1
                ? "Somebody nearby"
                : `${approaching.length} people nearby`,
    );

    let ring = $state<ReturnType<typeof Ring> | null>(null);
    let idle: HTMLElement;

    /**
     * Where a player's bar sits on the ring, in viewport coordinates.
     *
     * The flyer leaves from there rather than from the middle, so the animation tells the truth
     * about which bar was theirs. Before the ring's first painted frame there is no geometry to
     * read, so it falls back to the centre of the empty state — from nowhere in particular, but
     * from somewhere the user was already looking.
     */
    export function pointFor(player: NearbyPlayer): Point | null {
        if (!idle) return null;
        const geometry = ring?.geometry() ?? null;
        const canvas = ring?.element() ?? null;
        if (!geometry || !canvas) return Handoff.centreOf(idle);
        return Handoff.ringPoint(canvas, geometry, player.bearing);
    }

    const note = $derived(
        !connected
            ? "Nobody can hear you, and you cannot hear anyone. This clears itself when the link comes back."
            : approaching.length === 0
              ? "Voices appear here the moment someone walks into range. Nothing to join, nothing to dial."
              : "Close enough to see, not close enough to hear. They arrive on the list when they are.",
    );
</script>

<!--
  Not `Idle`: that one decides its own mode from whether it has sources, and this screen needs
  the distinction between one person approaching and several. In a proximity app you are alone
  constantly — at connect, while mining, anywhere off the beaten path — so this is somewhere
  users live rather than an edge case.

  It is also where a broken link surfaces, because clearing the roster is what the screen falls
  back to. One ring across all of it, in three registers `RingCast` already had: scanning while
  listening, reaching out to whoever is approaching, and at rest — genuinely at rest — when the
  link is down. The words underneath are what change, not the object.
-->
<div bind:this={idle} class="rad-idle" class:is-gone={gone} class:is-down={!connected}>
    <!-- Sized by CSS rather than by a `size` prop: the prop writes an inline width and height,
         which no stylesheet can then scale. `--rad-idle-ring` owns it instead. -->
    <Ring bind:this={ring} mode={cast.mode} sources={cast.sources} class="rad-idle__ring" />

    <span class="rad-idle__caption">
        <span class="rad-idle__kicker">{kicker}</span>
        <span class="rad-idle__headline">{headline}</span>
        <span class="rad-idle__note">{note}</span>

        {#if !connected && onstatus}
            <!-- The refresh action is gone, so this is the route from noticing to diagnosing.
                 It goes to the numbers rather than retrying blind. -->
            <button class="rad-btn rad-idle__action" onclick={onstatus}>{I18n.t("Connection status")}</button>
        {/if}
    </span>
</div>
