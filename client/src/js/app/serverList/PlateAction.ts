/**
 * What a plate's primary button does.
 *
 * `recheck` exists so a blocked server is never offered a doomed connect: a "connect
 * anyway" button on a server with no UDP path would only sell a failure as a choice.
 * `update` is the one mismatch this client can fix: it leads to the install, not a connection
 * that would fail. `blocked` leads nowhere.
 */
export type PlateAction = "connect" | "signin" | "recheck" | "update" | "blocked";
