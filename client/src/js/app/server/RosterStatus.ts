import type { ServerHealthStatus } from '../services/ServerHealthStatus';

/**
 * A row's state, including the one the health service has no name for: not asked yet.
 *
 * Rows appear before their checks finish, because a list that waited for the slowest
 * server would show nothing at all while one dead host times out.
 */
export type RosterStatus = ServerHealthStatus | 'checking';
