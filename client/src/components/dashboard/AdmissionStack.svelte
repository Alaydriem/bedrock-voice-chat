<script lang="ts">
    import { onDestroy, untrack } from "svelte";
    import { I18n } from "#lib/i18n/index.js";
    import {
        AdmissionStackView,
        type AdmissionCardState,
    } from "../../js/app/dashboard/AdmissionStackView";
    import AdmissionCard from "./AdmissionCard.svelte";

    interface Props {
        cards: readonly AdmissionCardState[];
        /** Requests beyond the cards shown. */
        waiting: number;
        groupName: string;
        oncheck: (joinerSocket: number) => void;
        ondecide: (joinerSocket: number, checkId: string, allow: boolean) => void;
        onlater: (joinerSocket: number) => void;
        ondismiss: (joinerSocket: number) => void;
    }
    let { cards, waiting, groupName, oncheck, ondecide, onlater, ondismiss }: Props = $props();

    /** How long a request lasts, to show the time left on its card. */
    const REQUEST_MS = 120_000;
    /** How long a check lasts once its code is shown. */
    const CHECK_MS = 60_000;

    const view = new AdmissionStackView();
    /** Bumped when the view's choice of open card changes, so the layout re-reads it. */
    let revision = $state(0);
    let now = $state(Date.now());
    const clock = setInterval(() => (now = Date.now()), 1_000);
    onDestroy(() => clearInterval(clock));

    /** When each card's code first appeared, for its time left. */
    const codeSeen = new Map<string, number>();

    $effect(() => {
        let changed = false;
        for (const card of cards) {
            if (card.phase === "code" && card.checkId && !codeSeen.has(card.checkId)) {
                codeSeen.set(card.checkId, Date.now());
                view.codeArrived(card.joinerSocket);
                changed = true;
            }
        }
        const live = new Set(cards.map((card) => card.joinerSocket));
        const open = view.expandedJoiner();
        if (open !== null && !live.has(open)) {
            view.forget(open);
            changed = true;
        }
        // Untracked: this effect writes the revision the layout reads, and must not re-run on it.
        if (changed) untrack(() => (revision += 1));
    });

    const laid = $derived.by(() => {
        void revision;
        return view.layout(cards, waiting);
    });

    function secondsLeft(card: AdmissionCardState): number | null {
        if (card.phase === "code" && card.checkId) {
            const seen = codeSeen.get(card.checkId) ?? now;
            return Math.max(0, Math.ceil((seen + CHECK_MS - now) / 1000));
        }
        if (card.phase === "request") {
            return Math.max(0, Math.ceil((card.receivedAt + REQUEST_MS - now) / 1000));
        }
        return null;
    }

    function show(joinerSocket: number): void {
        view.expand(joinerSocket);
        revision += 1;
    }
</script>

{#if laid.cards.length > 0}
    <div class="rad-admit" role="region" aria-label={I18n.t("Join requests")}>
        <div class="rad-admit__head">
            <span class="rad-label">{I18n.t("Join requests")}</span>
            <span class="rad-admit__count">{laid.cards.length + laid.more}</span>
        </div>
        {#each laid.cards as { card, expanded, canCheck } (card.joinerSocket)}
            <AdmissionCard
                joinerName={card.name}
                {groupName}
                phase={card.phase}
                {expanded}
                code={card.code}
                checkId={card.checkId}
                otherMember={card.otherMember}
                secondsLeft={secondsLeft(card)}
                {canCheck}
                pending={card.pending}
                deferred={card.deferred ?? false}
                oncheck={() => oncheck(card.joinerSocket)}
                ondecide={(checkId, allow) => ondecide(card.joinerSocket, checkId, allow)}
                onshow={() => show(card.joinerSocket)}
                onlater={() => onlater(card.joinerSocket)}
                ondismiss={() => ondismiss(card.joinerSocket)}
            />
        {/each}
        {#if laid.more > 0}
            <div class="rad-admit__head rad-admit__more">
                <span class="rad-label">{I18n.tf("+{count} more", { count: laid.more })}</span>
            </div>
        {/if}
    </div>
{/if}
