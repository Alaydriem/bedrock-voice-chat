<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import Verdict from "$radial/components/Verdict.svelte";
    import { PlateView } from "../../js/app/server/PlateView";
    import { PREFLIGHT_STEPS } from "../../js/app/server/preflight/PreflightStepName";
    import { PreflightVerdict } from "../../js/app/server/preflight/PreflightVerdict";
    import type { PreflightStepState } from "../../js/app/server/preflight/PreflightStepState";
    import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";
    import ServerIdentity from "./ServerIdentity.svelte";

    interface Props {
        /** The server being read, or null when the panel is closed. */
        entry: ServerRosterEntry | null;
        onclose: () => void;
        onrecheck: (server: string) => void;
        onremove: (entry: ServerRosterEntry) => void;
        onchoose: (server: string) => void;
    }
    let { entry, onclose, onrecheck, onremove, onchoose }: Props = $props();

    const ROW: Partial<Record<PreflightStepState, string>> = {
        running: "is-run",
        skipped: "is-skip",
        pending: "is-pending",
        ok: "is-ok",
        warn: "is-warn",
        bad: "is-bad",
    };

    let open = $derived(entry !== null);
    let view = $derived(entry ? PlateView.of(entry) : null);
    let verdict = $derived(entry ? PreflightVerdict.of(entry) : null);

    let total = $derived(entry ? entry.steps.reduce((sum, step) => sum + step.ms, 0) : 0);
    let ran = $derived(
        entry
            ? entry.steps.filter((step) => step.state !== "skipped" && step.state !== "pending")
                  .length
            : 0,
    );

    function duration(state: PreflightStepState, ms: number): string {
        if (state === "skipped") return "—";
        return ms ? `${ms} ms` : "…";
    }

    function onkeydown(event: KeyboardEvent): void {
        if (open && event.key === "Escape") onclose();
    }
</script>

<svelte:window {onkeydown} />

<div
    class="rad-scrim rad-scrim--modal {open ? 'is-on' : ''}"
    onclick={onclose}
    aria-hidden="true"
></div>

<!--
  Head and foot are fixed and only the readout between them scrolls, which is what the kit's
  panel class arranges. Kept in the DOM while closed so it animates out rather than vanishing.
-->
<div
    class="rad-modal rad-preflight-panel {open ? 'is-open' : ''}"
    role="dialog"
    aria-modal="true"
    aria-label={I18n.t("Preflight readout")}
    aria-hidden={!open}
>
    {#if entry && view && verdict}
        <div class="rad-preflight-panel__head">
            <ServerIdentity host={entry.host} avatarUrl={entry.avatarUrl} size={56} />
            <span class="rad-preflight-panel__text">
                <span class="rad-modal__title rad-preflight-panel__name">{entry.host}</span>
                <span class="rad-preflight-panel__host">signed in as {entry.player}</span>
            </span>
            <button class="rad-icon-btn" onclick={onclose} aria-label={I18n.t("Close")}>
                <Icon name="close" />
            </button>
        </div>

        <div class="rad-preflight-panel__body">
            <Verdict severity={verdict.severity} text={verdict.sentence} />

            <div class="rad-preflight-steps">
                {#each entry.steps as step (step.name)}
                    <div class="rad-preflight-step {ROW[step.state] ?? ''}">
                        <span class="rad-preflight-step__dot"></span>
                        <span>
                            <span class="rad-preflight-step__name">{step.name}</span>
                            <span class="rad-preflight-step__note">{step.note || "…"}</span>
                        </span>
                        <span class="rad-preflight-step__ms">
                            {duration(step.state, step.ms)}
                        </span>
                    </div>
                {/each}
            </div>

            <div class="rad-preflight-total">
                <span>{ran} of {PREFLIGHT_STEPS.length} checks ran</span>
                <span><b>{total} ms</b> total</span>
            </div>

            <!--
              Certificate expiry is checked in the first step and never named here. Days
              until expiry, the issuing CA and the rotation window are facts about mTLS, and
              the only thing anyone can act on is signing in again.
            -->
            <div class="rad-kv-grid" style="margin-top: 22px">
                <div class="rad-kv-group">
                    <div class="rad-kv-group__head">{I18n.t("Account")}</div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("Signed in as")}</span>
                        <span class="rad-kv__value">{entry.player}</span>
                    </div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("Game")}</span>
                        <span class="rad-kv__value">{entry.game}</span>
                    </div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("Stored in")}</span>
                        <span class="rad-kv__value">keyring · servers</span>
                    </div>
                </div>
                <div class="rad-kv-group">
                    <div class="rad-kv-group__head">{I18n.t("Link")}</div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("Round trip")}</span>
                        <span class="rad-kv__value">{entry.rtt ? `${entry.rtt} ms` : "—"}</span>
                    </div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("QUIC port")}</span>
                        <span class="rad-kv__value">
                            {entry.quicPort}{entry.quicPort === 443 ? "" : " (fallback)"}
                        </span>
                    </div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("Protocol")}</span>
                        <span class="rad-kv__value">
                            {entry.clientVersion || "—"} · server {entry.serverVersion ||
                                "—"}
                        </span>
                    </div>
                    <div class="rad-kv">
                        <span class="rad-kv__key">{I18n.t("Transport")}</span>
                        <span class="rad-kv__value">
                            {entry.status === "ws_fallback"
                                ? "WebSocket"
                                : "QUIC"} · TLS 1.3 · mTLS
                        </span>
                    </div>
                </div>
            </div>
        </div>

        <div class="rad-preflight-panel__foot">
            <button class="rad-btn rad-btn--danger" onclick={() => onremove(entry)}>
                <Icon name="trash" /> {I18n.t("Remove")}
            </button>
            <span class="rad-footbar__actions">
                <button class="rad-btn" onclick={() => onrecheck(entry.server)}>
                    <Icon name="refresh" /> {I18n.t("Recheck")}
                </button>
                <!--
                  A blocked server still leads somewhere from here: the update, not the
                  connection that would fail.
                -->
                <button
                    class="rad-btn rad-btn--primary"
                    onclick={() => onchoose(entry.server)}
                    disabled={entry.status === "checking" ||
                        (view.kind === "blocked" && !entry.clientTooOld)}
                >
                    {view.kind === "blocked" && entry.clientTooOld
                        ? "Update the client"
                        : view.action}
                </button>
            </span>
        </div>
    {/if}
</div>
