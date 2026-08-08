<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onMount } from "svelte";
    import type { ProxyServerEntry } from "../../js/app/managers/bedrock/ProxyServerEntry";
    import type { ProtocolVersionOption } from "../../js/bindings/ProtocolVersionOption";

    interface Props {
        /** The entry being edited, null to add, undefined for closed. */
        entry: ProxyServerEntry | null | undefined;
        /** Empty until the backend answers; the select falls back to Auto alone. */
        versions?: readonly ProtocolVersionOption[];
        onsave: (
            name: string,
            host: string,
            port: number,
            protocolVersion: number | undefined,
            id?: string,
        ) => void;
        oncancel: () => void;
    }
    let { entry, versions = [], onsave, oncancel }: Props = $props();

    const DEFAULT_PORT = 19132;
    /** Auto mirrors whatever the backend reports. */
    const AUTO = "auto";

    let name = $state("");
    let address = $state("");
    let version = $state(AUTO);
    let failure = $state("");

    // Re-seeded whenever the dialog opens on a different entry.
    let seeded: ProxyServerEntry | null | undefined;
    $effect(() => {
        if (entry === seeded) return;
        seeded = entry;
        name = entry?.name ?? "";
        address = entry ? `${entry.host}:${entry.port}` : "";
        version = entry?.protocolVersion != null ? String(entry.protocolVersion) : AUTO;
        failure = "";
    });

    /** Splits `host:port`, defaulting the port. An unparseable port returns null. */
    function parse(value: string): { host: string; port: number } | null {
        const trimmed = value.trim();
        if (!trimmed) return null;

        const colon = trimmed.lastIndexOf(":");
        if (colon <= 0) return { host: trimmed, port: DEFAULT_PORT };

        const host = trimmed.slice(0, colon);
        const port = Number(trimmed.slice(colon + 1));
        if (!Number.isInteger(port) || port < 1 || port > 65_535) return null;
        return { host, port };
    }

    function save(): void {
        const parsed = parse(address);
        if (!name.trim()) {
            failure = "Give it a name you will recognise in the list.";
            return;
        }
        if (!parsed) {
            failure = "That address does not look right. Use host or host:port.";
            return;
        }
        onsave(
            name.trim(),
            parsed.host,
            parsed.port,
            version === AUTO ? undefined : Number(version),
            entry?.id,
        );
    }

    let first = $state<HTMLInputElement | null>(null);
    onMount(() => first?.focus());
</script>

{#if entry !== undefined}
    <div class="rad-scrim rad-scrim--modal is-on"></div>
    <div class="rad-modal is-open">
        <h5 class="rad-modal__title">{entry ? "Edit this server" : "Add a server"}</h5>

        <p>
            Give it a name you will recognise, and the address Minecraft would have connected
            to. The port defaults to {DEFAULT_PORT}, and Auto reports whatever the backend does.
        </p>

        <span class="rad-input" style="margin-top: 14px; width: 100%">
            <input
                type="text"
                bind:value={name}
                bind:this={first}
                placeholder={I18n.t("Name")}
                aria-label={I18n.t("Name")}
            />
        </span>

        <span class="rad-input" style="margin-top: 10px; width: 100%">
            <input
                type="text"
                bind:value={address}
                placeholder="play.example.com"
                aria-label={I18n.t("Address")}
            />
        </span>

        <select
            class="rad-select"
            style="margin-top: 10px; width: 100%"
            bind:value={version}
            aria-label={I18n.t("Advertised version")}
        >
            <option value={AUTO}>{I18n.t("Auto — mirror the backend")}</option>
            {#each versions as option (option.protocol)}
                <option value={String(option.protocol)}>{option.label}</option>
            {/each}
        </select>

        {#if failure}
            <div class="rad-callout rad-callout--bad" style="margin-top: 12px">
                <span>{failure}</span>
            </div>
        {/if}

        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={oncancel}>{I18n.t("Cancel")}</button>
            <button class="rad-btn rad-btn--primary" onclick={save}>
                {entry ? "Save" : "Add"}
            </button>
        </div>
    </div>
{/if}
