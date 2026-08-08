import { I18n } from "$lib/i18n";
import { writable, type Readable, type Writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";
import type { KeybindConfig } from "../../../bindings/KeybindConfig";
import type { VoiceMode } from "../../../bindings/VoiceMode";
import type { KeybindRow } from "./KeybindRow";
import { AppStore } from "../../services/AppStore";

export class KeybindsManager {
    private readonly DEFAULT_CONFIG: KeybindConfig = {
        toggleMute: "ControlLeft+BracketLeft",
        toggleDeafen: "ControlLeft+BracketRight",
        toggleRecording: "ControlLeft+Backslash",
        pushToTalk: "Backquote",
        voiceMode: "openMic" as VoiceMode,
    };

    // Maps KeyboardEvent.code to our canonical key name (matching rdev mapping)
    private readonly CODE_MAP: Record<string, string> = {
        // Letters
        KeyA: "KeyA", KeyB: "KeyB", KeyC: "KeyC", KeyD: "KeyD", KeyE: "KeyE",
        KeyF: "KeyF", KeyG: "KeyG", KeyH: "KeyH", KeyI: "KeyI", KeyJ: "KeyJ",
        KeyK: "KeyK", KeyL: "KeyL", KeyM: "KeyM", KeyN: "KeyN", KeyO: "KeyO",
        KeyP: "KeyP", KeyQ: "KeyQ", KeyR: "KeyR", KeyS: "KeyS", KeyT: "KeyT",
        KeyU: "KeyU", KeyV: "KeyV", KeyW: "KeyW", KeyX: "KeyX", KeyY: "KeyY",
        KeyZ: "KeyZ",
        // Digits
        Digit0: "Digit0", Digit1: "Digit1", Digit2: "Digit2", Digit3: "Digit3",
        Digit4: "Digit4", Digit5: "Digit5", Digit6: "Digit6", Digit7: "Digit7",
        Digit8: "Digit8", Digit9: "Digit9",
        // Function keys
        F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
        F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
        // Punctuation
        Backquote: "Backquote", Minus: "Minus", Equal: "Equal",
        BracketLeft: "BracketLeft", BracketRight: "BracketRight",
        Backslash: "Backslash", Semicolon: "Semicolon", Quote: "Quote",
        Comma: "Comma", Period: "Period", Slash: "Slash",
        // Special
        Space: "Space", Tab: "Tab", CapsLock: "CapsLock", Enter: "Enter",
        Escape: "Escape", Backspace: "Backspace", Delete: "Delete",
        Insert: "Insert", Home: "Home", End: "End",
        PageUp: "PageUp", PageDown: "PageDown",
        ArrowUp: "ArrowUp", ArrowDown: "ArrowDown",
        ArrowLeft: "ArrowLeft", ArrowRight: "ArrowRight",
        PrintScreen: "PrintScreen", ScrollLock: "ScrollLock", Pause: "Pause",
        NumLock: "NumLock",
        // Numpad
        Numpad0: "Numpad0", Numpad1: "Numpad1", Numpad2: "Numpad2",
        Numpad3: "Numpad3", Numpad4: "Numpad4", Numpad5: "Numpad5",
        Numpad6: "Numpad6", Numpad7: "Numpad7", Numpad8: "Numpad8",
        Numpad9: "Numpad9",
        NumpadMultiply: "NumpadMultiply", NumpadAdd: "NumpadAdd",
        NumpadSubtract: "NumpadSubtract", NumpadDecimal: "NumpadDecimal",
        NumpadDivide: "NumpadDivide", NumpadEnter: "NumpadEnter",
    };

    // Display-friendly labels for key names
    private readonly DISPLAY_MAP: Record<string, string> = {
        ShiftLeft: "Shift", ControlLeft: "Ctrl", Alt: "Alt", MetaLeft: "Meta",
        BracketLeft: "[", BracketRight: "]", Backslash: "\\", Backquote: "`",
        Minus: "-", Equal: "=", Semicolon: ";", Quote: "'", Comma: ",",
        Period: ".", Slash: "/", Space: "Space", Tab: "Tab", Enter: "Enter",
        Escape: "Esc", Backspace: "Backspace", Delete: "Del", Insert: "Ins",
        ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
        NumpadMultiply: "Num *", NumpadAdd: "Num +", NumpadSubtract: "Num -",
        NumpadDecimal: "Num .", NumpadDivide: "Num /", NumpadEnter: "Num Enter",
    };

    private readonly MODIFIER_CODES = new Set([
        "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight",
        "AltLeft", "AltRight", "MetaLeft", "MetaRight",
    ]);

    public readonly rows: KeybindRow[] = [
        { id: "toggleMute", label: I18n.t("Toggle Mute") },
        { id: "toggleDeafen", label: I18n.t("Toggle Deafen") },
        { id: "toggleRecording", label: I18n.t("Toggle Recording") },
        { id: "pushToTalk", label: I18n.t("Push to Talk") },
    ];

    private isReadyStore: Writable<boolean>;
    public readonly isReady: Readable<boolean>;
    private configStore: Writable<KeybindConfig>;
    public readonly config: Readable<KeybindConfig>;
    private editingIdStore: Writable<keyof KeybindConfig | null>;
    public readonly editingId: Readable<keyof KeybindConfig | null>;
    private capturedComboStore: Writable<string>;
    public readonly capturedCombo: Readable<string>;
    private conflictErrorStore: Writable<string>;
    public readonly conflictError: Readable<string>;

    private store: Store | undefined = undefined;
    private currentConfig: KeybindConfig;
    private currentEditingId: keyof KeybindConfig | null = null;
    private readonly boundHandleKeyDown: (e: KeyboardEvent) => void;

    constructor() {
        this.currentConfig = { ...this.DEFAULT_CONFIG };

        this.isReadyStore = writable(false);
        this.isReady = { subscribe: this.isReadyStore.subscribe };
        this.configStore = writable({ ...this.DEFAULT_CONFIG });
        this.config = { subscribe: this.configStore.subscribe };
        this.editingIdStore = writable(null);
        this.editingId = { subscribe: this.editingIdStore.subscribe };
        this.capturedComboStore = writable("");
        this.capturedCombo = { subscribe: this.capturedComboStore.subscribe };
        this.conflictErrorStore = writable("");
        this.conflictError = { subscribe: this.conflictErrorStore.subscribe };

        this.boundHandleKeyDown = (e: KeyboardEvent) => {
            this.handleKeyDown(e);
        };
    }

    async initialize(): Promise<void> {
        this.store = await AppStore.load();
        const saved = await this.store.get<KeybindConfig>("keybinds");
        if (saved) {
            this.currentConfig = { ...this.DEFAULT_CONFIG, ...saved };
            this.configStore.set(this.currentConfig);
        }
        this.isReadyStore.set(true);
        document.addEventListener("keydown", this.boundHandleKeyDown);
    }

    displayCombo(combo: string): string {
        if (!combo) return I18n.t("Not set");
        return combo.split("+").map(part => {
            // Strip "Key" prefix for letters
            if (part.startsWith("Key") && part.length === 4) return part.charAt(3);
            // Strip "Digit" prefix
            if (part.startsWith("Digit") && part.length === 6) return part.charAt(5);
            // Strip "Numpad" prefix for numbers
            if (part.startsWith("Numpad") && part.length === 7) return "Num " + part.charAt(6);
            // F-keys as-is
            if (/^F\d+$/.test(part)) return part;
            return this.DISPLAY_MAP[part] || part;
        }).join(" + ");
    }

    private checkConflict(newCombo: string, excludeId: keyof KeybindConfig): string {
        for (const row of this.rows) {
            if (row.id === excludeId) continue;
            if ((this.currentConfig[row.id] as string) === newCombo) {
                return `Conflicts with "${row.label}"`;
            }
        }
        return "";
    }

    private handleKeyDown(e: KeyboardEvent): void {
        if (!this.currentEditingId) return;
        e.preventDefault();
        e.stopPropagation();

        // Ignore standalone modifier press
        if (this.MODIFIER_CODES.has(e.code)) {
            return;
        }

        const parts: string[] = [];
        // Canonical order: Ctrl, Alt, Shift, Meta
        if (e.ctrlKey) parts.push("ControlLeft");
        if (e.altKey) parts.push("Alt");
        if (e.shiftKey) parts.push("ShiftLeft");
        if (e.metaKey) parts.push("MetaLeft");

        const mapped = this.CODE_MAP[e.code];
        if (mapped) {
            parts.push(mapped);
        } else {
            parts.push(e.code);
        }

        const captured = parts.join("+");
        this.capturedComboStore.set(captured);

        const conflict = this.checkConflict(captured, this.currentEditingId);
        if (conflict) {
            this.conflictErrorStore.set(conflict);
            return;
        }

        this.conflictErrorStore.set("");
        (this.currentConfig as any)[this.currentEditingId] = captured;
        this.configStore.set(this.currentConfig);
        this.currentEditingId = null;
        this.editingIdStore.set(null);
        this.capturedComboStore.set("");
        void this.saveConfig();
    }

    startEditing(id: keyof KeybindConfig): void {
        this.currentEditingId = id;
        this.editingIdStore.set(id);
        this.capturedComboStore.set("");
        this.conflictErrorStore.set("");
    }

    cancelEditing(): void {
        this.currentEditingId = null;
        this.editingIdStore.set(null);
        this.capturedComboStore.set("");
        this.conflictErrorStore.set("");
    }

    resetBinding(id: keyof KeybindConfig): void {
        (this.currentConfig as any)[id] = (this.DEFAULT_CONFIG as any)[id];
        this.configStore.set(this.currentConfig);
        void this.saveConfig();
    }

    resetAll(): void {
        this.currentConfig = { ...this.DEFAULT_CONFIG, voiceMode: this.currentConfig.voiceMode };
        this.configStore.set(this.currentConfig);
        void this.saveConfig();
    }

    private async saveConfig(): Promise<void> {
        if (!this.store) return;
        await this.store.set("keybinds", this.currentConfig);
        await this.store.save();
        await invoke("start_keybind_listener", { config: this.currentConfig });
    }

    destroy(): void {
        document.removeEventListener("keydown", this.boundHandleKeyDown);
    }
}
