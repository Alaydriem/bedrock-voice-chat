/**
 * What a plate's primary button does.
 *
 * `recheck` exists so a blocked server is never offered a doomed connect: a "connect
 * anyway" button on a server with no UDP path would only sell a failure as a choice.
 * `blocked` leads somewhere too — an update, not a connection that would fail.
 */
export type PlateAction = 'connect' | 'signin' | 'recheck' | 'blocked';
