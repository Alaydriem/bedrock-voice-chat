import type { Permission } from "../../bindings/Permission";

export interface SettingsPane {
    readonly id: string;
    readonly title: string;
    /** Empty for the ungrouped run at the top of the list. */
    readonly group: string;
    /** Content is plates or a wide table, so the row measure does not apply. */
    readonly wide: boolean;
    /** Absent from the mobile build entirely. */
    readonly desktopOnly: boolean;
    /**
     * Hidden unless the viewer holds this permission.
     *
     * The gate is the permission set, not a boolean, so splitting `admin` into narrower
     * permissions later is a change to this one field and nothing else.
     */
    readonly requires?: Permission;
}

export interface SettingsGroup {
    readonly name: string;
    readonly panes: readonly SettingsPane[];
}
