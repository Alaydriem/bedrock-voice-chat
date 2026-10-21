/** One join request card, as the manager holds it. */
export interface AdmissionCardState {
    joinerSocket: number;
    /** The identity the server gave for the joiner's socket: a hint, not proof. */
    name: string;
    phase: "request" | "code" | "checking_elsewhere" | "admitted" | "failed";
    checkId: string | null;
    code: string | null;
    /** The member checking this joiner, when it is someone else. */
    otherMember: string | null;
    /** Check code was pressed and the code has not arrived yet. */
    pending: boolean;
    /** Unix milliseconds the request arrived. */
    receivedAt: number;
    /** Not now: the card shows its title only. */
    deferred?: boolean;
}

/** A card with what the stack decides about it. */
export interface LaidCard {
    card: AdmissionCardState;
    /** This card shows its code, Deny and Admit. At most one card does. */
    expanded: boolean;
    /** Check code may be pressed: fewer than three checks are open. */
    canCheck: boolean;
}

/**
 * Which join request card is open on its code.
 *
 * Several checks may run at once, but only one code is on screen at a time. With several codes
 * visible a member could compare a spoken code against the wrong card, or press Admit on the
 * one beside it.
 */
export class AdmissionStackView {
    static readonly MAX_CHECKS = 3;

    private expanded: number | null = null;

    expand(joinerSocket: number): void {
        this.expanded = joinerSocket;
    }

    /** The first code to arrive opens on its own; one that arrives while another is open waits. */
    codeArrived(joinerSocket: number): void {
        if (this.expanded === null) this.expanded = joinerSocket;
    }

    /** A card closed: if it was the open one, nothing is open now. */
    forget(joinerSocket: number): void {
        if (this.expanded === joinerSocket) this.expanded = null;
    }

    expandedJoiner(): number | null {
        return this.expanded;
    }

    layout(
        cards: readonly AdmissionCardState[],
        waiting: number,
    ): { cards: LaidCard[]; more: number } {
        const open = cards.filter((card) => card.pending || card.phase === "code").length;
        return {
            cards: cards.map((card) => ({
                card,
                expanded: card.phase === "code" && card.joinerSocket === this.expanded,
                canCheck: open < AdmissionStackView.MAX_CHECKS,
            })),
            more: waiting,
        };
    }
}
