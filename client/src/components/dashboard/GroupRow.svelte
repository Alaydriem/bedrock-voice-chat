<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import { SwipeActions } from "$radial/core/controllers/SwipeActions";
    import { GroupsView } from "../../js/app/dashboard/GroupsView";
    import type { GroupRowView } from "../../js/app/dashboard/GroupRowView";

    interface Props {
        group: GroupRowView;
        now: number;
        /** True when this row's tray is the open one. One at a time, owned by the parent. */
        open?: boolean;
        /** The row whose editor is open, if any. Also owned by the parent. */
        editId?: string | null;
        onjoin: (id: string) => void;
        onopen?: (id: string | null) => void;
        onedit?: (id: string | null) => void;
        onleave?: (id: string) => void;
        onclosegroup?: (id: string) => void;
        onrename?: (id: string, name: string) => void;
    }
    let {
        group,
        now,
        open = false,
        editId = null,
        onjoin,
        onopen,
        onedit,
        onleave,
        onclosegroup,
        onrename,
    }: Props = $props();

    /** Beyond this the cluster is a smear; the count carries the rest. */
    const SHOWN = 4;

    const shown = $derived(group.members.slice(0, SHOWN));
    const extra = $derived(Math.max(0, group.members.length - SHOWN));
    const since = $derived(GroupsView.since(group.activeAt, now));

    /** Leaving follows membership. Renaming and closing follow ownership. */
    const canLeave = $derived(group.joined);
    const canAdmin = $derived(group.owned);
    const hasActions = $derived(canLeave || canAdmin);

    let trayWidth = $state(0);
    let offset = $state(0);
    let dragging = $state(false);
    let startX = 0;
    let startedOpen = false;
    /**
     * Whether the pointer travelled far enough to be a swipe.
     *
     * Plain, not `$state`: it is read by the click handler that fires immediately after the
     * pointer lifts, to decide whether that click was a tap or the tail of a drag. Nothing
     * renders from it.
     */
    let swiped = false;

    // Settles to the parent's idea of open whenever no finger is down, so opening one row's
    // tray closes this one without either row knowing about the other.
    $effect(() => {
        if (!dragging) offset = SwipeActions.resting(open && hasActions, trayWidth);
    });

    /**
     * Editing is the parent's state, not this row's.
     *
     * Creating a group has to open its editor immediately, and a row cannot be told to open
     * itself before it exists. Holding it above the list means "the new one is being edited" is
     * something the list can say.
     */
    const editing = $derived(editId === group.id);
    let draft = $state("");
    let copied = $state(false);

    /**
     * Seeded on the way into editing only.
     *
     * `group` is a fresh object every second — the rows are re-derived on a clock so the activity
     * line stays current — so seeding the draft from `group.name` on every change would erase
     * what the user was typing once a second. Plain `let`, so setting it does not re-run this.
     */
    let seeded = false;
    $effect(() => {
        if (editing && !seeded) {
            draft = group.name;
            seeded = true;
        } else if (!editing) {
            seeded = false;
        }
    });

    function down(event: PointerEvent): void {
        if (!hasActions || editing) return;
        startX = event.clientX;
        startedOpen = open;
        swiped = false;
        dragging = true;
        // Captured so the rest of the gesture keeps arriving here after the finger has left the
        // row — which it will, because the row is moving out from under it.
        if (event.currentTarget instanceof Element) {
            event.currentTarget.setPointerCapture(event.pointerId);
        }
    }

    function move(event: PointerEvent): void {
        if (!dragging) return;
        const dx = event.clientX - startX;
        if (!swiped && !SwipeActions.isSwipe(dx)) return;
        swiped = true;
        offset = SwipeActions.offset(dx, trayWidth, startedOpen);
    }

    function up(): void {
        if (!dragging) return;
        dragging = false;
        if (!swiped) return;
        const latched = SwipeActions.latches(offset, trayWidth);
        onopen?.(latched ? group.id : null);
        offset = SwipeActions.resting(latched, trayWidth);
    }

    /**
     * A tap on the row joins or leaves. A tap that was really the end of a swipe does neither —
     * a gesture that reveals actions must not also take one.
     */
    function press(): void {
        if (swiped) {
            swiped = false;
            return;
        }
        if (open) {
            onopen?.(null);
            return;
        }
        onjoin(group.id);
    }

    function startEdit(): void {
        onopen?.(null);
        onedit?.(group.id);
    }

    function save(): void {
        const name = draft.trim();
        if (name && name !== group.name) onrename?.(group.id, name);
        onedit?.(null);
    }

    async function copyId(): Promise<void> {
        try {
            await navigator.clipboard.writeText(group.id);
            copied = true;
            setTimeout(() => (copied = false), 1600);
        } catch {
            copied = false;
        }
    }
</script>

<div class="rad-swipe" class:is-open={open && hasActions}>
    <!--
      The tray sits under the row rather than beside it, so revealing it costs no layout. It is
      rendered before the track for that reason, and only when there is something in it: a row
      you neither own nor belong to has no actions, and a swipe that uncovers nothing teaches
      the gesture does not work here.
    -->
    {#if hasActions}
        <div class="rad-swipe__tray" bind:clientWidth={trayWidth}>
            {#if canLeave}
                <button class="rad-swipe__action" onclick={() => onleave?.(group.id)}>
                    <Icon name="unlink" /> Leave
                </button>
            {/if}
            {#if canAdmin}
                <button class="rad-swipe__action" onclick={startEdit}>
                    <Icon name="gear" /> Edit
                </button>
                <button
                    class="rad-swipe__action rad-swipe__action--danger"
                    onclick={() => onclosegroup?.(group.id)}
                >
                    <Icon name="trash" /> Close
                </button>
            {/if}
        </div>
    {/if}

    <div
        class="rad-swipe__track"
        class:is-dragging={dragging}
        style="transform: translateX({offset}px)"
    >
        <!--
          No level meter. The server routes a channel's audio to its members alone, so a client
          outside one receives nothing to measure and a meter here would be an invention. The
          cluster says who is in it, dimmed for whoever cannot currently be heard, and the line
          underneath says when somebody last came or went.
        -->
        <!-- The gesture is bound to the row itself rather than to the track that moves it: the
             row is already a button, so the handlers land on something focusable and named. -->
        <button
            class="rad-group-row"
            class:is-on={group.joined}
            class:is-stirring={group.stirring}
            onclick={press}
            onpointerdown={down}
            onpointermove={move}
            onpointerup={up}
            onpointercancel={up}
        >
            <span class="rad-group-row__text">
                <span class="rad-group-row__name">{group.name}</span>
                {#if since}
                    <span class="rad-group-row__since">{since}</span>
                {/if}
            </span>

            <span class="rad-group-cluster">
                {#each shown as member (member.name)}
                    <!-- Letters rather than the block glyph the cards carry: at 22px a
                         mirrored 5x5 pattern is a smudge, and the question this cluster
                         answers is who is in there. -->
                    <span
                        class="rad-group-cluster__face"
                        style="--face: {member.hue}"
                        title={member.gamertag}
                    >
                        {member.initials}
                    </span>
                {/each}
                {#if extra}
                    <span class="rad-group-cluster__more">+{extra}</span>
                {/if}
            </span>

            <span class="rad-group-row__count">{group.members.length}</span>
        </button>
    </div>
</div>

{#if editing}
    <!--
      The id is here rather than behind a details view because it is the reason a group owner
      opens this panel at all: the mod is addressed by id, and an id you cannot copy is an id you
      have to transcribe from a screen.
    -->
    <div class="rad-group-edit">
        <label class="rad-group-edit__row">
            <span class="rad-label">Group name</span>
            <!-- `.rad-input` is the wrapper the kit styles; the field inside it is bare. -->
            <span class="rad-input">
                <input
                    bind:value={draft}
                    onkeydown={(e) => {
                        if (e.key === "Enter") save();
                        if (e.key === "Escape") onedit?.(null);
                    }}
                    aria-label="Group name"
                />
            </span>
        </label>

        <button class="rad-group-edit__id" onclick={copyId}>
            <span class="rad-label">Group id</span>
            <code>{group.id}</code>
            <span class="rad-group-edit__copy">
                <Icon name={copied ? "check" : "copy"} />
                {copied ? "Copied" : "Copy"}
            </span>
        </button>

        <div class="rad-group-edit__foot">
            <button class="rad-btn rad-btn--quiet" onclick={() => onedit?.(null)}>Cancel</button>
            <button class="rad-btn" onclick={save} disabled={draft.trim() === ""}>Save</button>
        </div>
    </div>
{/if}
