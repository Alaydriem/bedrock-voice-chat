import { I18n } from "#lib/i18n/index.js";

export type AddressFormatVerdict = { valid: true; host: string } | { valid: false; error: string };

/**
 * Whether a value typed into the server field can be a BVC server address at all.
 *
 * Answers before any request is made, so the player hears what is wrong with what they typed
 * rather than how the request it produced failed. The rules are ordered so the first message
 * that applies is the one most likely to explain the mistake.
 *
 * A BVC server presents a certificate issued to a domain name, so an IP address, a single
 * label or `localhost` cannot pass the TLS check even when something answers there.
 */
export class ServerAddressFormat {
    private static readonly LABEL = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/;
    private static readonly TLD = /^([a-z]{2,}|xn--[a-z0-9-]+)$/;
    private static readonly IPV4 = /^\d{1,3}(\.\d{1,3}){3}$/;
    private static readonly GAME_PORTS = new Set([19132, 19133]);

    static check(raw: string): AddressFormatVerdict {
        const value = raw
            .trim()
            .replace(/^https?:\/\//i, "")
            .replace(/\/$/, "");

        if (!value) {
            return ServerAddressFormat.invalid(I18n.t("Please enter a server URL"));
        }
        if (/\s/.test(value)) {
            return ServerAddressFormat.invalid(I18n.t("Server addresses cannot contain spaces"));
        }
        if (value.includes("@")) {
            return ServerAddressFormat.invalid(
                I18n.t(
                    "That looks like an email address. Enter the voice server's address, like voice.example.com",
                ),
            );
        }
        if (/[/?#]/.test(value)) {
            return ServerAddressFormat.invalid(
                I18n.t("Enter only the server address, without a path"),
            );
        }
        if (/^\d+$/.test(value)) {
            return ServerAddressFormat.invalid(I18n.t("That is a number, not a server address"));
        }

        const { host, port } = ServerAddressFormat.split(value);

        if (host.startsWith("[") || ServerAddressFormat.IPV4.test(host)) {
            return ServerAddressFormat.invalid(
                I18n.t("Enter the server's domain name, not an IP address"),
            );
        }
        if (port !== null) {
            const number = /^\d+$/.test(port) ? Number(port) : NaN;
            if (!(number >= 1 && number <= 65535)) {
                return ServerAddressFormat.invalid(I18n.t("Port must be between 1 and 65535"));
            }
            if (ServerAddressFormat.GAME_PORTS.has(number)) {
                return ServerAddressFormat.invalid(
                    I18n.t(
                        "19132 is the Minecraft game port. Ask the server owner for the voice server address",
                    ),
                );
            }
        }

        const lowered = host.toLowerCase();
        const labels = lowered.split(".");

        if (lowered === "localhost" || labels.length === 1) {
            return ServerAddressFormat.invalid(
                I18n.t("Enter the full server address, like voice.example.com"),
            );
        }
        if (!labels.every((label) => ServerAddressFormat.LABEL.test(label))) {
            return ServerAddressFormat.invalid(
                I18n.t("Server addresses can only use letters, numbers, dots and hyphens"),
            );
        }
        if (!ServerAddressFormat.TLD.test(labels[labels.length - 1])) {
            return ServerAddressFormat.invalid(
                I18n.t("Enter the full server address, like voice.example.com"),
            );
        }

        return { valid: true, host: port === null ? lowered : `${lowered}:${port}` };
    }

    private static invalid(error: string): AddressFormatVerdict {
        return { valid: false, error };
    }

    /** Host and port, by hand: `new URL` rewrites a numeric host into a dotted IPv4 address. */
    private static split(value: string): { host: string; port: string | null } {
        if (value.startsWith("[")) {
            const close = value.indexOf("]");
            if (close === -1) return { host: value, port: null };
            const rest = value.slice(close + 1);
            return {
                host: value.slice(0, close + 1),
                port: rest.startsWith(":") ? rest.slice(1) : null,
            };
        }

        const colon = value.lastIndexOf(":");
        if (colon === -1) return { host: value, port: null };
        // More than one colon without brackets is a bare IPv6 address.
        if (value.indexOf(":") !== colon) return { host: `[${value}]`, port: null };
        return { host: value.slice(0, colon), port: value.slice(colon + 1) };
    }
}
