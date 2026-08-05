export interface SettingsPane {
    readonly id: string;
    readonly title: string;
    /** Empty for the ungrouped run at the top of the list. */
    readonly group: string;
    /** Content is plates or a wide table, so the row measure does not apply. */
    readonly wide: boolean;
    /** Absent from the mobile build entirely. */
    readonly desktopOnly: boolean;
}

export interface SettingsGroup {
    readonly name: string;
    readonly panes: readonly SettingsPane[];
}
