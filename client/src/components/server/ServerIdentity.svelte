<script lang="ts">
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";

    interface Props {
        host: string;
        /** `avatar.png`, when the operator supplied one. Empty falls back to the glyph. */
        avatarUrl?: string;
        size?: number;
        /** Draw the glyph in over this many milliseconds, so a list assembles. */
        reveal?: number;
        class?: string;
    }
    let { host, avatarUrl = "", size = 56, reveal = 0, class: className = "" }: Props = $props();
</script>

<!--
  The glyph is the floor and the operator's art is the override, not the other way round:
  the glyph is the only one of the two guaranteed to exist and to agree across every client
  that knows the server's name.
-->
<span class="rad-server-id {className}" style="width: {size}px; height: {size}px">
    {#if avatarUrl}
        <img src={avatarUrl} alt="" />
    {:else}
        <ServerGlyph name={host} {size} {reveal} />
    {/if}
</span>
