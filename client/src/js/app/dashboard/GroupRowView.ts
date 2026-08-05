/** One member of a group, as the cluster draws them. */
export interface GroupMember {
    /** The CN form, which the glyph and the hue are both derived from. */
    name: string;
    gamertag: string;
    /** True when this client can currently hear them, so the face is not drawn dimmed. */
    audible: boolean;
}

/** One row in the groups pane. */
export interface GroupRowView {
    id: string;
    name: string;
    members: readonly GroupMember[];
    /** Whether this client is in it. */
    joined: boolean;
    /**
     * Whether this client created it.
     *
     * Not the same question as `joined`, and gating on the wrong one is what makes group
     * administration unusable: somebody coordinating several groups is in at most one of them at
     * a time, and still owns all of them. Renaming and closing follow ownership; leaving follows
     * membership.
     */
    owned: boolean;
    /**
     * Unix milliseconds of the last join or leave seen for this group, or null.
     *
     * The only activity signal available for a group you are not in: the server routes a
     * channel's audio to its members alone, so there is nothing to measure from outside. What
     * it can say honestly is when somebody last came or went.
     */
    activeAt: number | null;
    /** True while a join or leave is recent enough to be worth animating. */
    stirring: boolean;
}
