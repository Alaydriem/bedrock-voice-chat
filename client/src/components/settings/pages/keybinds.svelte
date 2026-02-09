<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { Store } from '@tauri-apps/plugin-store';
    import type { KeybindConfig } from "../../../js/bindings/KeybindConfig.ts";
    import type { VoiceMode } from "../../../js/bindings/VoiceMode.ts";

    const DEFAULT_CONFIG: KeybindConfig = {
        toggleMute: "BracketLeft",
        toggleDeafen: "BracketRight",
        toggleRecording: "Backslash",
        pushToTalk: "Backquote",
        voiceMode: "openMic" as VoiceMode,
    };

    // Maps KeyboardEvent.code to our canonical key name (matching rdev mapping)
    const CODE_MAP: Record<string, string> = {
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
    const DISPLAY_MAP: Record<string, string> = {
        ShiftLeft: "Shift", ControlLeft: "Ctrl", Alt: "Alt", MetaLeft: "Meta",
        BracketLeft: "[", BracketRight: "]", Backslash: "\\", Backquote: "`",
        Minus: "-", Equal: "=", Semicolon: ";", Quote: "'", Comma: ",",
        Period: ".", Slash: "/", Space: "Space", Tab: "Tab", Enter: "Enter",
        Escape: "Esc", Backspace: "Backspace", Delete: "Del", Insert: "Ins",
        ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
        NumpadMultiply: "Num *", NumpadAdd: "Num +", NumpadSubtract: "Num -",
        NumpadDecimal: "Num .", NumpadDivide: "Num /", NumpadEnter: "Num Enter",
    };

    const MODIFIER_CODES = new Set([
        "ShiftLeft", "ShiftRight", "ControlLeft", "ControlRight",
        "AltLeft", "AltRight", "MetaLeft", "MetaRight",
    ]);

    interface KeybindRow {
        id: keyof KeybindConfig;
        label: string;
    }

    const ROWS: KeybindRow[] = [
        { id: "toggleMute", label: "Toggle Mute" },
        { id: "toggleDeafen", label: "Toggle Deafen" },
        { id: "toggleRecording", label: "Toggle Recording" },
        { id: "pushToTalk", label: "Push to Talk" },
    ];

    let store: Store | undefined = $state(undefined);
    let isReady = $state(false);
    let config: KeybindConfig = $state({ ...DEFAULT_CONFIG });
    let editingId: keyof KeybindConfig | null = $state(null);
    let capturedCombo = $state("");
    let conflictError = $state("");

    function displayCombo(combo: string): string {
        if (!combo) return "Not set";
        return combo.split("+").map(part => {
            // Strip "Key" prefix for letters
            if (part.startsWith("Key") && part.length === 4) return part.charAt(3);
            // Strip "Digit" prefix
            if (part.startsWith("Digit") && part.length === 6) return part.charAt(5);
            // Strip "Numpad" prefix for numbers
            if (part.startsWith("Numpad") && part.length === 7) return "Num " + part.charAt(6);
            // F-keys as-is
            if (/^F\d+$/.test(part)) return part;
            return DISPLAY_MAP[part] || part;
        }).join(" + ");
    }

    function checkConflict(newCombo: string, excludeId: keyof KeybindConfig): string {
        for (const row of ROWS) {
            if (row.id === excludeId) continue;
            if ((config[row.id] as string) === newCombo) {
                return `Conflicts with "${row.label}"`;
            }
        }
        return "";
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (!editingId) return;
        e.preventDefault();
        e.stopPropagation();

        // Ignore standalone modifier press
        if (MODIFIER_CODES.has(e.code)) {
            return;
        }

        const parts: string[] = [];
        // Canonical order: Ctrl, Alt, Shift, Meta
        if (e.ctrlKey) parts.push("ControlLeft");
        if (e.altKey) parts.push("Alt");
        if (e.shiftKey) parts.push("ShiftLeft");
        if (e.metaKey) parts.push("MetaLeft");

        const mapped = CODE_MAP[e.code];
        if (mapped) {
            parts.push(mapped);
        } else {
            parts.push(e.code);
        }

        capturedCombo = parts.join("+");

        const conflict = checkConflict(capturedCombo, editingId);
        if (conflict) {
            conflictError = conflict;
            return;
        }

        conflictError = "";
        (config as any)[editingId] = capturedCombo;
        editingId = null;
        capturedCombo = "";
        saveConfig();
    }

    function startEditing(id: keyof KeybindConfig) {
        editingId = id;
        capturedCombo = "";
        conflictError = "";
    }

    function cancelEditing() {
        editingId = null;
        capturedCombo = "";
        conflictError = "";
    }

    function resetBinding(id: keyof KeybindConfig) {
        (config as any)[id] = (DEFAULT_CONFIG as any)[id];
        saveConfig();
    }

    function resetAll() {
        config = { ...DEFAULT_CONFIG, voiceMode: config.voiceMode };
        saveConfig();
    }

    async function saveConfig() {
        if (!store) return;
        await store.set("keybinds", config);
        await store.save();
        await invoke('start_keybind_listener', { config });
    }

    onMount(async () => {
        store = await Store.load("store.json", { autoSave: false });
        const saved = await store.get<KeybindConfig>("keybinds");
        if (saved) {
            config = { ...DEFAULT_CONFIG, ...saved };
        }
        isReady = true;
        document.addEventListener("keydown", handleKeyDown);
    });

    onDestroy(() => {
        document.removeEventListener("keydown", handleKeyDown);
    });
</script>

<div class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <div class="card px-5 pb-4 sm:px-5">
        <div class="my-3 flex flex-col">
            <h2 class="font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100 lg:text-base pb-2">
                Keyboard Shortcuts
            </h2>
            <p class="text-sm leading-6 hidden md:block">
                Configure global keyboard shortcuts for voice controls. These work even when BVC is not focused.
            </p>
        </div>

        {#if isReady}
        <div class="space-y-3 mt-2">
            {#each ROWS as row}
                {@const isEditing = editingId === row.id}
                {@const isHiddenInMode =
                    (row.id === "toggleMute" && config.voiceMode === "pushToTalk") ||
                    (row.id === "pushToTalk" && config.voiceMode === "openMic")}
                <div class="flex items-center justify-between py-3 px-4 rounded-lg transition-colors
                    {isEditing ? 'bg-primary/10 dark:bg-accent/15 ring-1 ring-primary/30 dark:ring-accent/30' : 'hover:bg-slate-50 dark:hover:bg-navy-600'}
                    {isHiddenInMode ? 'opacity-40' : ''}">
                    <div class="flex-1">
                        <span class="text-sm font-medium text-slate-700 dark:text-navy-100">
                            {row.label}
                        </span>
                        {#if isHiddenInMode}
                            <span class="ml-2 text-xs text-slate-400 dark:text-navy-300">
                                (not active in {config.voiceMode === "pushToTalk" ? "Push to Talk" : "Open Mic"} mode)
                            </span>
                        {/if}
                    </div>
                    <div class="flex items-center space-x-2">
                        {#if isEditing}
                            <span class="text-sm text-primary dark:text-accent-light animate-pulse">
                                {capturedCombo ? displayCombo(capturedCombo) : "Press a key combo..."}
                            </span>
                            {#if conflictError}
                                <span class="text-xs text-error">{conflictError}</span>
                            {/if}
                            <button
                                class="btn px-2 py-1 text-xs rounded bg-slate-200 hover:bg-slate-300 dark:bg-navy-500 dark:hover:bg-navy-400 text-slate-600 dark:text-navy-100"
                                onclick={cancelEditing}
                            >
                                Cancel
                            </button>
                        {:else}
                            <kbd class="px-2 py-1 text-sm font-mono bg-slate-100 dark:bg-navy-600 text-slate-700 dark:text-navy-100 rounded border border-slate-200 dark:border-navy-500">
                                {displayCombo(config[row.id] as string)}
                            </kbd>
                            <button
                                class="btn px-2 py-1 text-xs rounded bg-primary/10 hover:bg-primary/20 dark:bg-accent/15 dark:hover:bg-accent/25 text-primary dark:text-accent-light"
                                onclick={() => startEditing(row.id)}
                                disabled={isHiddenInMode}
                            >
                                Edit
                            </button>
                            <button
                                class="btn px-2 py-1 text-xs rounded bg-slate-200 hover:bg-slate-300 dark:bg-navy-500 dark:hover:bg-navy-400 text-slate-600 dark:text-navy-100"
                                onclick={() => resetBinding(row.id)}
                                disabled={isHiddenInMode}
                            >
                                Reset
                            </button>
                        {/if}
                    </div>
                </div>
            {/each}
        </div>

        <div class="my-4 h-px bg-slate-200 dark:bg-navy-500"></div>

        <div class="flex justify-end">
            <button
                class="btn px-4 py-2 text-sm rounded-lg bg-slate-200 hover:bg-slate-300 dark:bg-navy-500 dark:hover:bg-navy-400 text-slate-700 dark:text-navy-100"
                onclick={resetAll}
            >
                Reset All Keybinds
            </button>
        </div>
        {/if}
    </div>
</div>
