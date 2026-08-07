/**
 * Where adding a server leads, and the way back out of it.
 *
 * Sign-in, not the roster. The roster is where saved servers are chosen between, and a device
 * with one saved server never stops there — it is forwarded to that server's dashboard before
 * the roster draws. Sending "add a server" to the roster therefore returns the user to the
 * screen they pressed it on, which is the one place they were trying to leave.
 *
 * The way back is the roster rather than either endpoint, because it is the answer that holds
 * whichever way the sign-in went: it lists both servers once one was added, and forwards to the
 * only one when the sign-in was abandoned.
 */
export class AddServerRoute {
    /** The roster, and the only `return` target this app honours. */
    static readonly RETURN_TO = '/';

    static readonly HREF = `/login?addserver=true&return=${AddServerRoute.RETURN_TO}`;

    /**
     * The way off a sign-in screen, for someone who reached it from a screen they were already
     * using rather than from a cold launch.
     *
     * The label names no screen. Going back leads through the roster, which one saved server
     * passes straight through, so "back to the server list" would promise a screen most
     * people never see.
     */
    static backFrom(params: URLSearchParams): { href: string; label: string } {
        if (params.has('addserver') && params.get('return') === AddServerRoute.RETURN_TO) {
            return { href: AddServerRoute.RETURN_TO, label: 'Cancel' };
        }
        return { href: '/dashboard', label: 'Back to Dashboard' };
    }
}
