import type { AudioFileResponse } from "../../bindings/AudioFileResponse";
import { RecordingsView } from "./RecordingsView";
import type { SoundRow } from "./SoundRow";

/** The server's sound library, as the table shows it, and who may change it. */
export class SoundLibraryView {
    static readonly UPLOAD = "audio_upload";
    static readonly DELETE = "audio_delete";

    static rows(files: readonly AudioFileResponse[]): readonly SoundRow[] {
        return files.map((file) => this.row(file));
    }

    static row(file: AudioFileResponse): SoundRow {
        return {
            id: file.id,
            name: file.original_filename,
            uploader: this.uploader(file),
            added: this.added(file.created_at),
            length: RecordingsView.duration(file.duration_ms),
            size: this.size(file.file_size_bytes),
        };
    }

    /** Which field carries the name depends on the game, so all three are tried. */
    static uploader(file: AudioFileResponse): string {
        const identity = file.uploader as unknown as Record<string, unknown>;
        const name = identity?.gamertag ?? identity?.name ?? identity?.player;
        return typeof name === "string" && name.trim() ? name : "Unknown";
    }

    /** Seconds since the epoch, in the reader's locale. */
    static added(createdAt: number | bigint): string {
        const seconds = Number(createdAt);
        if (!Number.isFinite(seconds) || seconds <= 0) return "—";
        return new Date(seconds * 1000).toLocaleDateString();
    }

    /** Kilobytes until a megabyte. */
    static size(bytes: number | bigint): string {
        const value = Number(bytes);
        if (!Number.isFinite(value) || value <= 0) return "0 KB";
        const kb = value / 1024;
        return kb >= 1024 ? `${(kb / 1024).toFixed(1)} MB` : `${Math.round(kb)} KB`;
    }

    static canUpload(permissions: readonly string[]): boolean {
        return permissions.includes(this.UPLOAD);
    }

    static canDelete(permissions: readonly string[]): boolean {
        return permissions.includes(this.DELETE);
    }

    /** Managing means deleting; uploading is a button above the table. */
    static canManage(permissions: readonly string[]): boolean {
        return this.canDelete(permissions);
    }

    static pageCount(total: number, pageSize: number): number {
        if (pageSize <= 0) return 1;
        return Math.max(1, Math.ceil(total / pageSize));
    }

    /** The backend counts pages from zero. Asking for page 1 of two items returns none. */
    static readonly FIRST_PAGE = 0;

    static clampPage(page: number, total: number, pageSize: number): number {
        return Math.min(Math.max(this.FIRST_PAGE, page), this.pageCount(total, pageSize) - 1);
    }

    /**
     * A display name for a picked file.
     *
     * A desktop pick is a path, so its basename is the name. An Android pick is a
     * `content://` URI whose last segment is usually a row id — `audio%3A1000000123` — and
     * the only place the real name exists is the provider, which `resolved` carries when it
     * answered.
     */
    static fileNameFrom(picked: string, resolved?: string | null): string {
        const fromProvider = this.clean(resolved ?? "");
        if (fromProvider) return fromProvider;

        const last = picked.split(/[\\/]/).pop() ?? "";
        let decoded = last;
        try {
            decoded = decodeURIComponent(last);
        } catch {
            // Not percent-encoded.
        }
        return this.clean(decoded.split(":").pop() ?? "") ?? "upload.ogg";
    }

    /** A candidate is a name only if it carries an extension to encode by. */
    private static clean(candidate: string): string | null {
        const trimmed = candidate.trim();
        return /^[^.]+\.[A-Za-z0-9]{1,5}$/.test(trimmed) ? trimmed : null;
    }
}
