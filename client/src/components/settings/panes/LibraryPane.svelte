<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { open } from "@tauri-apps/plugin-dialog";
    import { readFile } from "@tauri-apps/plugin-fs";
    import { Store } from "@tauri-apps/plugin-store";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { AudioFileResponse } from "../../../js/bindings/AudioFileResponse";
    import type { PaginatedResponse } from "../../../js/bindings/PaginatedResponse";
    import type { ListState } from "../../../js/app/settings/ListState";
    import type { SoundRow } from "../../../js/app/settings/SoundRow";
    import { SoundLibraryView } from "../../../js/app/settings/SoundLibraryView";
    import { SoundPreview } from "../../../js/app/settings/SoundPreview";
    import ListShell from "../ListShell.svelte";

    const PAGE_SIZE = 8;

    let listState = $state<ListState>("loading");
    let rows = $state<readonly SoundRow[]>([]);
    let permissions = $state<readonly string[]>([]);
    let failure = $state("");
    let search = $state("");
    // Local preview only. Playing into the world is the jukebox's job, in game.
    const preview = new SoundPreview((fileId) =>
        invoke<string>("get_audio_stream_url", { fileId }),
    );
    let playing = $state<string | null>(null);
    $effect(() => preview.playing.subscribe((v) => (playing = v)));

    // Zero-based, as the backend counts them.
    let page = $state(SoundLibraryView.FIRST_PAGE);
    let total = $state(0);
    let deleting = $state<SoundRow | null>(null);
    let uploading = $state(false);
    let uploadError = $state("");

    const canUpload = $derived(SoundLibraryView.canUpload(permissions));
    const canManage = $derived(SoundLibraryView.canManage(permissions));
    const pages = $derived(SoundLibraryView.pageCount(total, PAGE_SIZE));

    async function loadPermissions(): Promise<void> {
        try {
            const store = await Store.load("store.json", { autoSave: false, defaults: {} });
            const server = await store.get<string>("current_server");
            if (!server) return;
            const raw = await invoke<string>("get_credential", {
                server,
                key: "server_permissions",
            });
            permissions = raw ? (JSON.parse(raw).allowed ?? []) : [];
        } catch {
            // A failed read grants nothing.
            permissions = [];
        }
    }

    async function load(): Promise<void> {
        preview.stop();
        listState = "loading";
        try {
            const result = await invoke<PaginatedResponse<AudioFileResponse>>("list_audio_files", {
                query: {
                    page,
                    page_size: PAGE_SIZE,
                    sort_by: "created_at",
                    sort_order: "desc",
                    search: search.trim() || null,
                },
            });
            rows = SoundLibraryView.rows(result.items);
            total = result.total;
            listState = "ready";
        } catch (e) {
            failure = e instanceof Error ? e.message : String(e);
            listState = "failed";
        }
    }

    onDestroy(() => preview.stop());

    onMount(async () => {
        await loadPermissions();
        await load();
    });

    // Searching resets the page.
    function onsearch(value: string): void {
        search = value;
        page = SoundLibraryView.FIRST_PAGE;
        void load();
    }

    function goToPage(next: number): void {
        page = SoundLibraryView.clampPage(next, total, PAGE_SIZE);
        void load();
    }

    async function copyId(id: string): Promise<void> {
        await navigator.clipboard?.writeText(id).catch(() => {});
    }

    /** Opens the platform picker, uploads, then reloads the current page of results. */
    async function upload(): Promise<void> {
        uploading = true;
        uploadError = "";
        try {
            const picked = await open({
                multiple: false,
                directory: false,
                filters: [{ name: "Audio", extensions: ["ogg", "mp3", "wav", "flac", "m4a"] }],
            });
            if (typeof picked !== "string") return;

            // Read here rather than in Rust: Android's picker returns a `content://` URI,
            // which no filesystem call can open. The fs plugin resolves it.
            const bytes = await readFile(picked);
            // The picker's URI rarely carries the name; the content provider has it.
            const resolved = await invoke<string | null>("resolve_display_name", {
                path: picked,
            }).catch(() => null);
            await invoke("upload_audio_bytes", {
                bytes: Array.from(bytes),
                fileName: SoundLibraryView.fileNameFrom(picked, resolved),
            });
            await load();
        } catch (e) {
            uploadError = e instanceof Error ? e.message : String(e);
        } finally {
            uploading = false;
        }
    }

    async function runDelete(): Promise<void> {
        if (!deleting) return;
        const fileId = deleting.id;
        deleting = null;
        await invoke("delete_audio_file", { fileId }).catch(() => {});
        await load();
    }
</script>

<div class="rad-section">
    {#if canUpload}
        <div class="rad-card">
            <div class="rad-row">
                <span class="rad-row__text">
                    <span class="rad-row__label">{I18n.t("Add a sound")}</span>
                    <span class="rad-row__note">{I18n.t("Uploaded sounds can be played on a Jukeboxes")}</span>
                </span>
                <span class="rad-row__control">
                    <button
                        class="rad-btn rad-btn--primary"
                        disabled={uploading}
                        onclick={() => void upload()}
                    >
                        <Icon name="plus" />
                        {uploading ? "Uploading…" : "Upload…"}
                    </button>
                </span>
            </div>
        </div>

        {#if uploadError}
            <div class="rad-callout rad-callout--bad"><span>{uploadError}</span></div>
        {/if}
    {/if}

    <div class="rad-swatchrow" style="margin-bottom: 4px">
        <span class="rad-search" style="flex: 1 1 220px">
            <Icon name="search" />
            <input
                type="search"
                placeholder={I18n.t("Search sounds")}
                aria-label={I18n.t("Search sounds")}
                value={search}
                oninput={(e) => onsearch((e.target as HTMLInputElement).value)}
            />
        </span>
        <StatusChip>{total} sound{total === 1 ? "" : "s"}</StatusChip>
    </div>

    <ListShell
        state={listState}
        count={rows.length}
        failTitle="Couldn't load the library"
        failNote={failure || "The server did not answer when we asked for its sounds."}
        onretry={() => void load()}
        emptyTitle={search ? "Nothing matches that" : "No sounds yet"}
        emptyNote={search
            ? "No sound on this server has that in its name."
            : "Upload one and it becomes available to the jukebox for everyone on this server."}
    >
        <div class="rad-card">
            <!-- Table and cards render the same `rows`; the container query in table.css
                 picks one. -->
            <div class="rad-table-wrap rad-table-wrap--wide">
                <table class="rad-table">
                    <thead>
                        <tr>
                            <th>{I18n.t("Sound")}</th>
                            <th>{I18n.t("Uploaded by")}</th>
                            <th class="rad-num">{I18n.t("Added")}</th>
                            <th class="rad-num">{I18n.t("Length")}</th>
                            <th class="rad-num">{I18n.t("Size")}</th>
                            <th class="rad-num">ID</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {#each rows as row (row.id)}
                            <tr>
                                <td><span class="rad-table__name">{row.name}</span></td>
                                <td>{row.uploader}</td>
                                <td class="rad-num">{row.added}</td>
                                <td class="rad-num">{row.length}</td>
                                <td class="rad-num">{row.size}</td>
                                <td class="rad-num rad-table__id">{row.id}</td>
                                <td class="rad-table__actions">
                                    <span class="rad-row-actions">
                                        <span
                                            class="rad-tip"
                                            data-rad-tip={playing === row.id
                                                ? "Stop"
                                                : "Preview locally"}
                                        >
                                            <button
                                                class="rad-kebab"
                                                class:is-on={playing === row.id}
                                                onclick={() => void preview.toggle(row.id)}
                                                aria-label={playing === row.id
                                                    ? `Stop ${row.name}`
                                                    : `Play ${row.name}`}
                                            >
                                                <Icon name={playing === row.id ? "stop" : "play"} />
                                            </button>
                                        </span>
                                        <span class="rad-tip" data-rad-tip="Copy sound id">
                                            <button
                                                class="rad-kebab"
                                                onclick={() => void copyId(row.id)}
                                                aria-label={I18n.t("Copy id")}
                                            >
                                                <Icon name="copy" />
                                            </button>
                                        </span>
                                        {#if canManage}
                                            <button
                                                class="rad-kebab"
                                                onclick={() => (deleting = row)}
                                                aria-label={I18n.tf("Delete {name}", { name: row.name })}
                                            >
                                                <Icon name="trash" />
                                            </button>
                                        {/if}
                                    </span>
                                </td>
                            </tr>
                        {/each}
                    </tbody>
                </table>
            </div>

            <div class="rad-datacards">
                {#each rows as row (row.id)}
                    <div class="rad-datacard">
                        <span class="rad-datacard__name">{row.name}</span>
                        <span class="rad-datacard__id">{row.id}</span>
                        <span class="rad-datacard__meta">
                            <span>{row.length}</span>
                            <span>{row.size}</span>
                            <span>{row.uploader}</span>
                            <span>{row.added}</span>
                        </span>
                        <span class="rad-datacard__actions">
                            <button
                                class="rad-kebab"
                                class:is-on={playing === row.id}
                                onclick={() => void preview.toggle(row.id)}
                                aria-label={playing === row.id
                                    ? `Stop ${row.name}`
                                    : `Play ${row.name}`}
                            >
                                <Icon name={playing === row.id ? "stop" : "play"} />
                            </button>
                            <button
                                class="rad-kebab"
                                onclick={() => void copyId(row.id)}
                                aria-label={I18n.tf("Copy id for {name}", { name: row.name })}
                            >
                                <Icon name="copy" />
                            </button>
                            {#if canManage}
                                <button
                                    class="rad-kebab"
                                    onclick={() => (deleting = row)}
                                    aria-label={I18n.tf("Delete {name}", { name: row.name })}
                                >
                                    <Icon name="trash" />
                                </button>
                            {/if}
                        </span>
                    </div>
                {/each}
            </div>

            {#if pages > 1}
                <div class="rad-pager">
                    <span>{total} sound{total === 1 ? "" : "s"}</span>
                    <span class="rad-pager__pages">
                        <button
                            disabled={page === SoundLibraryView.FIRST_PAGE}
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
                        <button
                            disabled={page >= pages - 1}
                            onclick={() => goToPage(page + 1)}>›</button
                        >
                    </span>
                </div>
            {/if}
        </div>
    </ListShell>

    <div class="rad-callout">
        <span>
            {I18n.t("In game, run")} <code>/bvc:disc &lt;id&gt;</code> to give yourself a music disc that plays the sound.
        </span>
    </div>
</div>

{#if deleting}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">{I18n.t("Delete this sound?")}</h5>
        <p>
            <b>{deleting.name}</b> is removed for everyone on this server. Anything that plays it by
            id stops working.
        </p>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => (deleting = null)}>{I18n.t("Keep it")}</button>
            <button class="rad-btn rad-btn--danger" onclick={() => void runDelete()}>
                <Icon name="trash" /> {I18n.t("Delete")}
            </button>
        </div>
    </div>
{/if}
