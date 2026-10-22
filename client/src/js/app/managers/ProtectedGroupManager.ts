import { I18n } from "#lib/i18n/index.js";
import { type Readable, type Writable, writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { error as logError } from "@charlesportwoodii/tauri-plugin-curia";
import type { ProtectedGroupCheckingNotice } from "../../bindings/ProtectedGroupCheckingNotice";
import type { ProtectedGroupCodeNotice } from "../../bindings/ProtectedGroupCodeNotice";
import type { ProtectedGroupEpochNotice } from "../../bindings/ProtectedGroupEpochNotice";
import type { ProtectedGroupJoinStartedNotice } from "../../bindings/ProtectedGroupJoinStartedNotice";
import type { ProtectedGroupJoinerNotice } from "../../bindings/ProtectedGroupJoinerNotice";
import type { ProtectedGroupLostNotice } from "../../bindings/ProtectedGroupLostNotice";
import type { ProtectedGroupRequestNotice } from "../../bindings/ProtectedGroupRequestNotice";
import type { ProtectedGroupRosterNotice } from "../../bindings/ProtectedGroupRosterNotice";
import type { ProtectedGroupWaitingNotice } from "../../bindings/ProtectedGroupWaitingNotice";
import type { AdmissionCardState } from "../dashboard/AdmissionStackView";

interface GroupData {
    in_group: boolean;
    id: string | null;
    name: string | null;
}

/** How turning on "Private Group" ended. */
export type ProtectResult = "ok" | "not_alone" | "already_protected" | "failed";

/** Where the join dialog is. */
export type JoinPhase =
    | "ask"
    | "waiting"
    | "code"
    | "joining"
    | "refused"
    | "mismatch"
    | "expired"
    | "no_admitter";

/** The join dialog's state: one request at a time. */
export interface JoinFlow {
    id: string;
    phase: JoinPhase;
    code: string | null;
    checkId: string | null;
    /** The canonical identity of the member checking this client. */
    checker: string | null;
}

/**
 * The webview's view of this client's end-to-end encrypted group: its roster and safety code,
 * the join dialog while this client asks to join, and the join request cards while it is a
 * member others ask.
 *
 * Everything here comes from the app's Rust side, which holds the group's keys and runs the
 * checks. Nothing that could decrypt the group ever reaches this page.
 */
export class ProtectedGroupManager {
    static readonly JOINED = "protected-group:joined";
    static readonly ROSTER = "protected-group:roster";
    static readonly EPOCH = "protected-group:epoch";
    static readonly LOST = "protected-group:lost";
    static readonly REQUEST = "protected-group:request";
    static readonly WAITING = "protected-group:waiting";
    static readonly CODE = "protected-group:code";
    static readonly CHECKING = "protected-group:checking";
    static readonly CHECK_FAILED = "protected-group:check-failed";
    static readonly REQUEST_ENDED = "protected-group:request-ended";
    static readonly ADMITTED = "protected-group:admitted";
    static readonly JOIN_STARTED = "protected-group:join-started";
    /** How long an admitted joiner's card stays green before it closes. */
    static readonly ADMITTED_MS = 4_000;
    /** The endings the join dialog shows itself, rather than as a notice. */
    private static readonly DIALOG_ENDINGS: readonly JoinPhase[] = [
        "refused",
        "mismatch",
        "expired",
        "no_admitter",
    ];

    private readonly rosterStore: Writable<string[]>;
    private readonly epochStore: Writable<number | null>;
    private readonly safetyCodeStore: Writable<string | null>;
    private readonly lostReasonStore: Writable<string | null>;
    private readonly activeStore: Writable<boolean>;
    private readonly cardsStore: Writable<AdmissionCardState[]>;
    private readonly waitingStore: Writable<number>;
    private readonly joinFlowStore: Writable<JoinFlow | null>;
    private unlisteners: (() => void)[] = [];
    private flow: JoinFlow | null = null;

    public readonly roster: Readable<string[]>;
    public readonly epoch: Readable<number | null>;
    public readonly safetyCode: Readable<string | null>;
    public readonly lostReason: Readable<string | null>;
    public readonly active: Readable<boolean>;
    public readonly cards: Readable<AdmissionCardState[]>;
    public readonly waiting: Readable<number>;
    public readonly joinFlow: Readable<JoinFlow | null>;

    constructor() {
        this.rosterStore = writable([]);
        this.epochStore = writable(null);
        this.safetyCodeStore = writable(null);
        this.lostReasonStore = writable(null);
        this.activeStore = writable(false);
        this.cardsStore = writable([]);
        this.waitingStore = writable(0);
        this.joinFlowStore = writable(null);

        this.roster = { subscribe: this.rosterStore.subscribe };
        this.epoch = { subscribe: this.epochStore.subscribe };
        this.safetyCode = { subscribe: this.safetyCodeStore.subscribe };
        this.lostReason = { subscribe: this.lostReasonStore.subscribe };
        this.active = { subscribe: this.activeStore.subscribe };
        this.cards = { subscribe: this.cardsStore.subscribe };
        this.waiting = { subscribe: this.waitingStore.subscribe };
        this.joinFlow = { subscribe: this.joinFlowStore.subscribe };
    }

    /** The copy for a lost reason. Every string is a literal so the catalog extractor finds it. */
    static lostMessage(reason: string): string {
        switch (reason) {
            case "refused":
                return I18n.t("A member did not admit you");
            case "mismatch":
                return I18n.t("The codes did not match");
            case "expired":
                return I18n.t("No member answered");
            case "no_admitter":
                return I18n.t("No member is online");
            default:
                return I18n.t("Connection to the group was lost. Rejoin to continue.");
        }
    }

    async startListening(): Promise<void> {
        const view = getCurrentWebviewWindow();
        this.unlisteners = await Promise.all([
            view.listen<ProtectedGroupEpochNotice>(ProtectedGroupManager.JOINED, (event) => {
                this.lostReasonStore.set(null);
                this.activeStore.set(true);
                this.setFlow(null);
                this.applyEpoch(event.payload);
            }),
            view.listen<ProtectedGroupEpochNotice>(ProtectedGroupManager.EPOCH, (event) =>
                this.applyEpoch(event.payload),
            ),
            view.listen<ProtectedGroupRosterNotice>(ProtectedGroupManager.ROSTER, (event) =>
                this.rosterStore.set([...event.payload.members]),
            ),
            view.listen<ProtectedGroupLostNotice>(ProtectedGroupManager.LOST, (event) =>
                this.onLost(event.payload.reason),
            ),
            view.listen<ProtectedGroupRequestNotice>(ProtectedGroupManager.REQUEST, (event) =>
                this.onRequest(event.payload),
            ),
            view.listen<ProtectedGroupWaitingNotice>(ProtectedGroupManager.WAITING, (event) =>
                this.waitingStore.set(event.payload.count),
            ),
            view.listen<ProtectedGroupCodeNotice>(ProtectedGroupManager.CODE, (event) =>
                this.onCode(event.payload),
            ),
            view.listen<ProtectedGroupCheckingNotice>(ProtectedGroupManager.CHECKING, (event) =>
                this.updateCard(event.payload.joiner_socket, (card) =>
                    // This member's own check or admission wins over a late announcement.
                    card.pending || card.phase === "code" || card.phase === "admitted"
                        ? card
                        : {
                              ...card,
                              phase: "checking_elsewhere",
                              otherMember: event.payload.member_name,
                          },
                ),
            ),
            view.listen<ProtectedGroupJoinerNotice>(ProtectedGroupManager.CHECK_FAILED, (event) =>
                this.updateCard(event.payload.joiner_socket, (card) => ({
                    ...card,
                    phase: "failed",
                    pending: false,
                    code: null,
                })),
            ),
            view.listen<ProtectedGroupJoinerNotice>(ProtectedGroupManager.REQUEST_ENDED, (event) =>
                this.removeCard(event.payload.joiner_socket),
            ),
            view.listen<ProtectedGroupJoinerNotice>(ProtectedGroupManager.ADMITTED, (event) => {
                const joiner = event.payload.joiner_socket;
                this.updateCard(joiner, (card) => ({ ...card, phase: "admitted", code: null }));
                setTimeout(() => this.removeCard(joiner), ProtectedGroupManager.ADMITTED_MS);
            }),
            view.listen<ProtectedGroupJoinStartedNotice>(ProtectedGroupManager.JOIN_STARTED, (event) =>
                this.setFlow({
                    id: event.payload.channel_id,
                    phase: "waiting",
                    code: null,
                    checkId: null,
                    checker: null,
                }),
            ),
        ]);
    }

    /**
     * Turns on "Private Group" for the owner's own group, while they are alone in it.
     * `not_alone` and `already_protected` are the server's refusals; anything else failed, and
     * the server closes the group if this client never starts it.
     */
    async protect(id: string): Promise<ProtectResult> {
        try {
            this.lostReasonStore.set(null);
            await invoke<GroupData>("group_protect", { id });
            return "ok";
        } catch (e) {
            const reason = String(e);
            if (reason === "not_alone" || reason === "already_protected") return reason;
            logError(`ProtectedGroupManager: protect failed: ${e}`);
            return "failed";
        }
    }

    /** Asks to join. Admission finishes later, as a joined or lost event. */
    async join(id: string): Promise<boolean> {
        try {
            this.lostReasonStore.set(null);
            await invoke<GroupData>("group_join_protected", { id });
            return true;
        } catch (e) {
            logError(`ProtectedGroupManager: join failed: ${e}`);
            return false;
        }
    }

    /** Opens the join dialog on a protected group, before anything is sent. */
    openJoin(id: string): void {
        this.setFlow({ id, phase: "ask", code: null, checkId: null, checker: null });
    }

    /** "Ask to join" or "Ask again": sends a fresh request. */
    async ask(): Promise<void> {
        const flow = this.flow;
        if (!flow) return;
        this.setFlow({ ...flow, phase: "waiting", code: null, checkId: null, checker: null });
        if (!(await this.join(flow.id))) this.setFlow(null);
    }

    /**
     * "It matches" or "No match" for the code on screen.
     *
     * "Joining" is shown before the call: "It matches" releases the held Welcome, so the joined
     * event can arrive before the call returns, and setting it afterwards would reopen a dialog
     * that event already closed.
     */
    async confirm(matches: boolean): Promise<void> {
        const flow = this.flow;
        if (!flow || flow.phase !== "code" || !flow.checkId) return;
        const joining: JoinFlow | null = matches ? { ...flow, phase: "joining" } : null;
        if (joining) this.setFlow(joining);
        try {
            await invoke("group_admission_confirm", { checkId: flow.checkId, matches });
        } catch (e) {
            logError(`ProtectedGroupManager: confirm failed: ${e}`);
            if (joining && this.flow === joining) this.setFlow(flow);
        }
    }

    closeJoin(): void {
        this.setFlow(null);
    }

    /** "Check code" on a card: starts this member's check of that joiner. */
    async check(joinerSocket: number): Promise<void> {
        this.updateCard(joinerSocket, (card) => ({ ...card, pending: true, deferred: false }));
        try {
            await invoke("group_admission_check", { joinerSocket });
        } catch (e) {
            logError(`ProtectedGroupManager: check failed: ${e}`);
            this.updateCard(joinerSocket, (card) => ({ ...card, pending: false }));
        }
    }

    /** Admit or Deny on the card showing `checkId`. Only that check's id is sent. */
    async decide(joinerSocket: number, checkId: string, allow: boolean): Promise<void> {
        try {
            await invoke("group_admission_decide", { joinerSocket, checkId, allow });
        } catch (e) {
            logError(`ProtectedGroupManager: decide failed: ${e}`);
        }
    }

    /** Not now: the card folds to its title until the request ends or is checked. */
    later(joinerSocket: number): void {
        this.updateCard(joinerSocket, (card) => ({ ...card, deferred: true }));
    }

    dismiss(joinerSocket: number): void {
        this.removeCard(joinerSocket);
    }

    /**
     * Wipes this client's keys for the group, ahead of leaving its channel. The microphone stays
     * silent until `release`, which the caller sends only once the channel is left.
     */
    async stop(): Promise<void> {
        try {
            await invoke("group_stop_protected");
        } catch (e) {
            logError(`ProtectedGroupManager: stop failed: ${e}`);
        }
        this.reset();
    }

    /** Lets audio flow unsealed again, after the protected channel has been left. */
    async release(): Promise<void> {
        try {
            await invoke("group_release_protected");
        } catch (e) {
            logError(`ProtectedGroupManager: release failed: ${e}`);
        }
    }

    cleanup(): void {
        for (const unlisten of this.unlisteners) unlisten();
        this.unlisteners = [];
    }

    private onRequest(notice: ProtectedGroupRequestNotice): void {
        this.cardsStore.update((cards) =>
            cards.some((card) => card.joinerSocket === notice.joiner_socket)
                ? cards
                : [
                      ...cards,
                      {
                          joinerSocket: notice.joiner_socket,
                          name: notice.name,
                          phase: "request",
                          checkId: null,
                          code: null,
                          otherMember: null,
                          pending: false,
                          receivedAt: Date.now(),
                      },
                  ],
        );
    }

    /**
     * A member's code lands only on the card for the same joiner socket, never by name. A card
     * already showing a live code keeps it: a different check id cannot swap it in place.
     */
    private onCode(notice: ProtectedGroupCodeNotice): void {
        if (notice.role === "joiner") {
            const flow = this.flow;
            if (!flow) return;
            this.setFlow({
                ...flow,
                phase: "code",
                code: notice.code,
                checkId: notice.check_id,
                checker: notice.peer_name,
            });
            return;
        }
        this.updateCard(notice.joiner_socket, (card) => {
            if (card.phase === "code" && card.checkId && card.checkId !== notice.check_id) {
                return card;
            }
            return {
                ...card,
                phase: "code",
                pending: false,
                checkId: notice.check_id,
                code: notice.code,
            };
        });
    }

    private onLost(reason: string): void {
        this.reset();
        const flow = this.flow;
        const ending = reason as JoinPhase;
        if (flow && ProtectedGroupManager.DIALOG_ENDINGS.includes(ending)) {
            this.setFlow({ ...flow, phase: ending });
            return;
        }
        this.setFlow(null);
        this.lostReasonStore.set(reason);
    }

    private updateCard(
        joinerSocket: number,
        change: (card: AdmissionCardState) => AdmissionCardState,
    ): void {
        this.cardsStore.update((cards) =>
            cards.map((card) => (card.joinerSocket === joinerSocket ? change(card) : card)),
        );
    }

    private removeCard(joinerSocket: number): void {
        this.cardsStore.update((cards) => cards.filter((card) => card.joinerSocket !== joinerSocket));
    }

    private setFlow(flow: JoinFlow | null): void {
        this.flow = flow;
        this.joinFlowStore.set(flow);
    }

    private applyEpoch(notice: ProtectedGroupEpochNotice): void {
        this.epochStore.set(notice.epoch);
        this.safetyCodeStore.set(notice.safety_code);
    }

    private reset(): void {
        this.activeStore.set(false);
        this.rosterStore.set([]);
        this.epochStore.set(null);
        this.safetyCodeStore.set(null);
        this.cardsStore.set([]);
        this.waitingStore.set(0);
    }
}
