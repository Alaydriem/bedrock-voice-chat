<script lang="ts">
    import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import Segmented from "$radial/components/Segmented.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import ListShell from "../ListShell.svelte";
    import { ViewerPermissionsManager } from "../../../js/app/managers/ViewerPermissionsManager";
    import type { ViewerIdentity } from "../../../js/app/managers/ViewerIdentity";
    import type { ListState } from "../../../js/app/settings/ListState";
    import type { ManagedPlayerRow } from "../../../js/app/settings/ManagedPlayerRow";
    import { ManagePlayersView } from "../../../js/app/settings/ManagePlayersView";
    import type { AdminActionOutcome } from "../../../js/bindings/AdminActionOutcome";
    import type { AdminUserRow } from "../../../js/bindings/AdminUserRow";
    import type { Game } from "../../../js/bindings/Game";
    import type { PaginatedResponse } from "../../../js/bindings/PaginatedResponse";
    import type { Permission } from "../../../js/bindings/Permission";
    import type { PermissionEntry } from "../../../js/bindings/PermissionEntry";
    import type { PermissionListResponse } from "../../../js/bindings/PermissionListResponse";

    const viewer = new ViewerPermissionsManager();

    /**
     * The operator reading this screen, so their own row can hide what cannot be done to it.
     *
     * Null until the introspect on mount answers. Every row keeps its ban button until then,
     * which is the safe direction: the server refuses a self-ban regardless.
     */
    let me = $state<ViewerIdentity | null>(null);
    $effect(() => viewer.identity.subscribe((value) => (me = value)));

    let listState = $state<ListState>("loading");
    let rows = $state<readonly ManagedPlayerRow[]>([]);
    let total = $state(0);
    let page = $state(ManagePlayersView.FIRST_PAGE);
    let search = $state("");
    let failure = $state("");

    /** Set when the server refuses the roster and introspect confirms the permission is gone. */
    let revoked = $state(false);
    /** The last refusal, in words. Cleared by the next successful action. */
    let notice = $state("");

    let banning = $state<ManagedPlayerRow | null>(null);
    let adding = $state(false);
    /** Focused when the dialog opens: it holds one field, and typing is the only thing to do. */
    let gamertagField = $state<HTMLInputElement | null>(null);
    $effect(() => {
        if (adding) gamertagField?.focus();
    });
    let newGamertag = $state("");
    /**
     * Every player this build can whitelist is a Minecraft player, and it is the only game
     * `Game` carries. Adding a second one is putting a picker back around this value.
     */
    const newGame: Game = "minecraft";
    let addError = $state("");

    let open = $state<ManagedPlayerRow | null>(null);
    let overrides = $state<readonly PermissionEntry[]>([]);

    const pages = $derived(ManagePlayersView.pageCount(total, ManagePlayersView.PAGE_SIZE));

    async function load(): Promise<void> {
        listState = "loading";
        try {
            const result = await invoke<PaginatedResponse<AdminUserRow>>("admin_list_users", {
                query: {
                    page,
                    page_size: ManagePlayersView.PAGE_SIZE,
                    search: search.trim() || null,
                    // Unfiltered by game. With one game shipping, a filter would separate
                    // the roster from itself; the route still takes the parameter.
                    game: null,
                },
            });
            rows = ManagePlayersView.rows(result.items);
            total = result.total;
            listState = "ready";
        } catch (e) {
            // A refused roster is either a lost permission or a broken connection, and the
            // two need different words. Introspect is what tells them apart.
            const live = await viewer.refresh();
            if (!ViewerPermissionsManager.has(live, "admin")) {
                revoked = true;
                return;
            }
            const detail = e instanceof Error ? e.message : String(e);
            failure =
                detail.trim() ||
                I18n.t("The server did not answer when we asked for its players.");
            listState = "failed";
        }
    }

    onMount(async () => {
        // Refreshed rather than read from the cache: this pane needs the server anyway, and
        // introspect is what names the identity its certificate proved. A failure falls back
        // to the cached permissions and the cached gamertag.
        await viewer.refresh();
        await load();
    });

    function onsearch(value: string): void {
        search = value;
        page = ManagePlayersView.FIRST_PAGE;
        void load();
    }

    function goToPage(next: number): void {
        page = ManagePlayersView.clampPage(next, total, ManagePlayersView.PAGE_SIZE);
        void load();
    }

    async function runBan(): Promise<void> {
        const target = banning;
        banning = null;
        if (!target) return;

        notice = "";
        const outcome = await invoke<AdminActionOutcome>("admin_set_banished", {
            gamertag: target.gamertag,
            game: target.game,
            banish: !target.banned,
        }).catch(() => "invalid" as AdminActionOutcome);

        if (outcome !== "applied") {
            notice = ManagePlayersView.banFailure(outcome);
            if (outcome === "forbidden") revoked = true;
            return;
        }
        await load();
    }

    function openAdd(): void {
        addError = "";
        newGamertag = "";
        adding = true;
    }

    async function addPlayer(): Promise<void> {
        addError = "";
        const gamertag = newGamertag.trim();
        if (!gamertag) return;

        const outcome = await invoke<AdminActionOutcome>("admin_create_user", {
            gamertag,
            game: newGame,
        }).catch(() => "invalid" as AdminActionOutcome);

        if (outcome !== "applied") {
            addError = ManagePlayersView.addFailure(outcome);
            if (outcome === "forbidden") revoked = true;
            return;
        }
        newGamertag = "";
        adding = false;
        await load();
    }

    async function openRow(row: ManagedPlayerRow): Promise<void> {
        open = row;
        overrides = [];
        try {
            const response = await invoke<PermissionListResponse>("admin_list_permissions", {
                gamertag: row.gamertag,
                game: row.game,
            });
            overrides = response.entries;
        } catch {
            // An unreadable override list leaves every segment on Default, which is what a
            // player with no overrides looks like. Writing one still works.
            overrides = [];
        }
    }

    /**
     * Default clears the override; allow and deny write one.
     *
     * Two routes, because "no override, follow the server default" is a different state
     * from "explicitly denied" and only the server decides what the default is.
     */
    async function setState(permission: Permission, next: string): Promise<void> {
        if (!open) return;
        notice = "";
        const target = open;

        const outcome =
            next === "default"
                ? await invoke<AdminActionOutcome>("admin_clear_permission", {
                      gamertag: target.gamertag,
                      game: target.game,
                      permission,
                  }).catch(() => "invalid" as AdminActionOutcome)
                : await invoke<AdminActionOutcome>("admin_set_permission", {
                      gamertag: target.gamertag,
                      game: target.game,
                      permission,
                      effect: next,
                  }).catch(() => "invalid" as AdminActionOutcome);

        // Clearing an override that was never there answers 404, and the end state is
        // exactly what the operator asked for.
        if (outcome !== "applied" && !(next === "default" && outcome === "not_found")) {
            notice = ManagePlayersView.permissionFailure(outcome);
            if (outcome === "forbidden") revoked = true;
            return;
        }

        await openRow(target);
        await load();
    }
</script>

{#if revoked}
    <div class="rad-section">
        <div class="rad-callout rad-callout--bad">
            <span>{I18n.t("You no longer hold the admin permission.")}</span>
        </div>
    </div>
{:else}
    <div class="rad-section">
        <div class="rad-card">
            <div class="rad-row">
                <span class="rad-row__text">
                    <span class="rad-row__label">{I18n.t("Add a player")}</span>
                    <span class="rad-row__note">
                        {I18n.t("Only players on this list can sign in to this server.")}
                    </span>
                </span>
                <span class="rad-row__control">
                    <button class="rad-btn rad-btn--primary" onclick={openAdd}>
                        <Icon name="plus" /> {I18n.t("Add player")}
                    </button>
                </span>
            </div>
        </div>

        {#if notice}
            <div class="rad-callout rad-callout--bad"><span>{notice}</span></div>
        {/if}

        <div class="rad-swatchrow" style="margin-bottom: 4px">
            <span class="rad-search" style="flex: 1 1 220px">
                <Icon name="search" />
                <input
                    type="search"
                    placeholder={I18n.t("Search players")}
                    aria-label={I18n.t("Search players")}
                    value={search}
                    oninput={(e) => onsearch((e.target as HTMLInputElement).value)}
                />
            </span>
            <StatusChip>{total} {total === 1 ? I18n.t("player") : I18n.t("players")}</StatusChip>
        </div>

        <ListShell
            state={listState}
            count={rows.length}
            failTitle={I18n.t("Couldn't load the players")}
            failNote={failure}
            onretry={() => void load()}
            emptyTitle={search ? I18n.t("Nothing matches that") : I18n.t("No players yet")}
            emptyNote={search
                ? I18n.t("No player on this server has that in their name.")
                : I18n.t("Add one and they can sign in to this server.")}
        >
            <div class="rad-card">
                <div class="rad-table-wrap rad-table-wrap--wide">
                    <table class="rad-table">
                        <thead>
                            <tr>
                                <th></th>
                                <th>{I18n.t("Player")}</th>
                                <th>{I18n.t("Game")}</th>
                                <th class="rad-num">{I18n.t("Added")}</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each rows as row (row.key)}
                                <tr>
                                    <td>
                                        <!-- Presence, then one slot per permission, and the
                                             way into the editor. A button rather than a
                                             decorated span: it does something, and its label
                                             has to say both what it opens and what it shows,
                                             because the colours are the only other telling. -->
                                        <button
                                            class="rad-matrix__blocks"
                                            onclick={() => void openRow(row)}
                                            aria-label={I18n.tf("Permissions for {name} — {state}", {
                                                name: row.gamertag,
                                                state: ManagePlayersView.blocksLabel(row),
                                            })}
                                        >
                                            {#each ManagePlayersView.blocks(row) as block (block.label)}
                                                <i style="background:{block.color}"></i>
                                            {/each}
                                        </button>
                                    </td>
                                    <td><span class="rad-table__name">{row.gamertag}</span></td>
                                    <td>{ManagePlayersView.gameLabel(row.game)}</td>
                                    <td class="rad-num">{row.added}</td>
                                    <td class="rad-table__actions">
                                        <span class="rad-row-actions">
                                            <button
                                                class="rad-kebab"
                                                onclick={() => void openRow(row)}
                                                aria-label={I18n.tf("Settings for {name}", {
                                                    name: row.gamertag,
                                                })}
                                            >
                                                <Icon name="gear" />
                                            </button>
                                            <!-- No ban button on your own row. The server
                                                 refuses a self-ban with a 409, so offering
                                                 it is offering a refusal. -->
                                            {#if !ManagePlayersView.isSelf(row, me)}
                                                <button
                                                    class="rad-kebab"
                                                    onclick={() => (banning = row)}
                                                    aria-label={row.banned
                                                        ? I18n.tf("Unban {name}", {
                                                              name: row.gamertag,
                                                          })
                                                        : I18n.tf("Ban {name}", {
                                                              name: row.gamertag,
                                                          })}
                                                >
                                                    <Icon name={row.banned ? "check" : "close"} />
                                                </button>
                                            {/if}
                                        </span>
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>

                <!-- The same rows for a narrow container, which table.css switches to at
                     620px. Name first, then the identical strip, cog and ban: the state a
                     colour carries on the table is also written out here, because a strip
                     is small on a phone and the meta line has room for the word. -->
                <div class="rad-datacards">
                    {#each rows as row (row.key)}
                        <div class="rad-datacard">
                            <span class="rad-datacard__name">{row.gamertag}</span>
                            <span class="rad-datacard__meta">
                                <span>{ManagePlayersView.statusLabel(row.status)}</span>
                                <span>{ManagePlayersView.gameLabel(row.game)}</span>
                                <span>{row.added}</span>
                            </span>
                            <span class="rad-datacard__actions">
                                <button
                                    class="rad-matrix__blocks"
                                    onclick={() => void openRow(row)}
                                    aria-label={I18n.tf("Permissions for {name} — {state}", {
                                        name: row.gamertag,
                                        state: ManagePlayersView.blocksLabel(row),
                                    })}
                                >
                                    {#each ManagePlayersView.blocks(row) as block (block.label)}
                                        <i style="background:{block.color}"></i>
                                    {/each}
                                </button>
                                <button
                                    class="rad-kebab"
                                    onclick={() => void openRow(row)}
                                    aria-label={I18n.tf("Settings for {name}", {
                                        name: row.gamertag,
                                    })}
                                >
                                    <Icon name="gear" />
                                </button>
                                {#if !ManagePlayersView.isSelf(row, me)}
                                    <button
                                        class="rad-kebab"
                                        onclick={() => (banning = row)}
                                        aria-label={row.banned
                                            ? I18n.tf("Unban {name}", { name: row.gamertag })
                                            : I18n.tf("Ban {name}", { name: row.gamertag })}
                                    >
                                        <Icon name={row.banned ? "check" : "close"} />
                                    </button>
                                {/if}
                            </span>
                        </div>
                    {/each}
                </div>

                {#if pages > 1}
                    <div class="rad-pager">
                        <span>{total}</span>
                        <span class="rad-pager__pages">
                            <button
                                disabled={page === ManagePlayersView.FIRST_PAGE}
                                onclick={() => goToPage(page - 1)}>‹</button
                            >
                            {#each Array.from({ length: pages }, (_, i) => i) as index (index)}
                                <button
                                    class={index === page ? "is-on" : ""}
                                    onclick={() => goToPage(index)}
                                >
                                    {index + 1}
                                </button>
                            {/each}
                            <button disabled={page >= pages - 1} onclick={() => goToPage(page + 1)}
                                >›</button
                            >
                        </span>
                    </div>
                {/if}
            </div>
        </ListShell>
    </div>
{/if}

{#if adding}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">{I18n.t("Add a player")}</h5>
        <p>
            {I18n.t(
                "Their gamertag, spelled as Xbox spells it. Only players on this list can sign in to this server.",
            )}
        </p>

        <span class="rad-input" style="margin-top: 14px; width: 100%">
            <input
                type="text"
                bind:value={newGamertag}
                bind:this={gamertagField}
                placeholder={I18n.t("Gamertag")}
                aria-label={I18n.t("Gamertag")}
                onkeydown={(e) => {
                    if (e.key === "Enter" && newGamertag.trim()) void addPlayer();
                }}
            />
        </span>

        {#if addError}
            <div class="rad-callout rad-callout--bad" style="margin-top: 12px">
                <span>{addError}</span>
            </div>
        {/if}

        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (adding = false)}>{I18n.t("Cancel")}</button>
            <button
                class="rad-btn rad-btn--primary"
                disabled={!newGamertag.trim()}
                onclick={() => void addPlayer()}
            >
                {I18n.t("Add")}
            </button>
        </div>
    </div>
{/if}

{#if banning}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">
            {banning.banned ? I18n.t("Unban this player?") : I18n.t("Ban this player?")}
        </h5>
        <p>
            {#if banning.banned}
                <b>{banning.gamertag}</b>
                {I18n.t(
                    "can sign in again. They are issued a new certificate on their next sign-in; the revoked one stays revoked.",
                )}
            {:else}
                <b>{banning.gamertag}</b>
                {I18n.t(
                    "loses voice access now. Their certificate is revoked and their live session is closed, so unbanning them requires them to sign in again.",
                )}
            {/if}
        </p>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (banning = null)}>{I18n.t("Cancel")}</button>
            <button class="rad-btn rad-btn--danger" onclick={() => void runBan()}>
                {banning.banned ? I18n.t("Unban") : I18n.t("Ban")}
            </button>
        </div>
    </div>
{/if}

{#if open}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">
            {I18n.tf("Permissions for {name}", { name: open.gamertag })}
        </h5>
        <p>{I18n.t("Default follows this server's configuration. Allow and deny override it.")}</p>
        <!-- Stacked: three segments and a label like "Administrator" do not share one
             row without the label being crushed into it. -->
        {#each ManagePlayersView.EDITABLE as permission (permission)}
            <div class="rad-row rad-row--stack">
                <span class="rad-row__text">
                    <span class="rad-row__label">{ManagePlayersView.label(permission)}</span>
                </span>
                <span class="rad-row__control">
                    <Segmented
                        label={ManagePlayersView.label(permission)}
                        value={ManagePlayersView.state(overrides, permission)}
                        options={[
                            { value: "default", label: I18n.t("Default") },
                            { value: "allow", label: I18n.t("Allow") },
                            { value: "deny", label: I18n.t("Deny") },
                        ]}
                        onchange={(next) => void setState(permission, next)}
                    />
                </span>
            </div>
        {/each}
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (open = null)}>{I18n.t("Done")}</button>
        </div>
    </div>
{/if}
