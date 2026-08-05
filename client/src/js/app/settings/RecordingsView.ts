import type { RecordingSession } from "../../bindings/RecordingSession";
import type { RecordingRow } from "./RecordingRow";

/** Recordings, as the table shows them. */
export class RecordingsView {
    static rows(sessions: readonly RecordingSession[]): readonly RecordingRow[] {
        return sessions.map((session) => this.row(session));
    }

    static row(session: RecordingSession): RecordingRow {
        const manifest = session.session_data;
        const started = Number(manifest.start_timestamp);
        const named = manifest.name?.trim();
        const bytes = session.file_size_mb * 1024 * 1024;

        return {
            id: manifest.session_id,
            name: named || this.recordedLabel(started),
            unnamed: !named,
            recorded: this.recordedLabel(started),
            recordedAt: started,
            length: this.duration(manifest.duration_ms),
            players: manifest.participants,
            tracks: manifest.participants.length,
            size: this.size(bytes),
            bytes,
            exportable: session.exportable,
        };
    }

    /** When it happened, in the reader's locale. Identifies an unnamed recording. */
    static recordedLabel(startedMs: number): string {
        if (!Number.isFinite(startedMs) || startedMs <= 0) return "Unknown date";
        const at = new Date(startedMs);
        return `${at.toLocaleDateString()} ${at.toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
        })}`;
    }

    /** Hours only when there are hours. */
    static duration(ms: bigint | number | null): string {
        const total = Math.floor(Number(ms ?? 0) / 1000);
        if (!Number.isFinite(total) || total <= 0) return "—";
        const hours = Math.floor(total / 3600);
        const minutes = Math.floor((total % 3600) / 60);
        const seconds = total % 60;
        const pad = (n: number) => String(n).padStart(2, "0");
        return hours > 0 ? `${hours}:${pad(minutes)}:${pad(seconds)}` : `${minutes}:${pad(seconds)}`;
    }

    /** Megabytes until a gigabyte. */
    static size(bytes: number): string {
        if (!Number.isFinite(bytes) || bytes <= 0) return "0 MB";
        const mb = bytes / (1024 * 1024);
        return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`;
    }

    static totalSize(rows: readonly RecordingRow[]): string {
        return this.size(rows.reduce((sum, row) => sum + row.bytes, 0));
    }
}
