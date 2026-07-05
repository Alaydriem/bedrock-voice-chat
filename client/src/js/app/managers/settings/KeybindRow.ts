import type { KeybindConfig } from "../../../bindings/KeybindConfig";

export interface KeybindRow {
    id: keyof KeybindConfig;
    label: string;
}
