<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import type { NetworkInterface } from "../../js/bindings/NetworkInterface";
    import { ListenAddress } from "../../js/app/settings/ListenAddress";

    interface Props {
        interfaces: readonly NetworkInterface[];
        port: number;
        /** What the socket is bound to. `0.0.0.0` means the list, anything else one address. */
        bind: string;
        /** Prefix for a copied value, such as `ws://`. */
        scheme?: string;
        singleLabel: string;
        singleNote: string;
        listLabel: string;
        listNote: string;
        /** Include loopback, for a client that may be on this machine. */
        includeLoopback?: boolean;
        empty?: string;
    }
    let {
        interfaces,
        port,
        bind,
        scheme = "",
        singleLabel,
        singleNote,
        listLabel,
        listNote,
        includeLoopback = false,
        empty = "No network address yet.",
    }: Props = $props();

    const every = $derived(bind === ListenAddress.ANY);
    const single = $derived(`${scheme}${ListenAddress.join(bind, port)}`);
    const candidates = $derived(ListenAddress.candidates(interfaces, port, includeLoopback));

    async function copy(text: string): Promise<void> {
        await navigator.clipboard?.writeText(text).catch(() => {});
    }
</script>

{#if every}
    <SettingRow label={listLabel} note={listNote} stack>
        <div class="rad-addresses">
            {#each candidates as candidate (candidate.address)}
                <div class="rad-address">
                    <span class="rad-address__value">{scheme}{candidate.address}</span>
                    <span class="rad-address__label">{candidate.label}</span>
                    <button
                        class="rad-icon-btn"
                        onclick={() => void copy(`${scheme}${candidate.address}`)}
                        aria-label={I18n.tf("Copy {scheme}{address}", { scheme, address: candidate.address })}
                    >
                        <Icon name="copy" />
                    </button>
                </div>
            {:else}
                <span class="rad-address__label">{empty}</span>
            {/each}
        </div>
    </SettingRow>
{:else}
    <SettingRow label={singleLabel} note={singleNote}>
        {#snippet control()}
            <span class="rad-input" style="width: 190px">
                <input type="text" value={single} readonly aria-label={singleLabel} />
            </span>
            <button
                class="rad-icon-btn"
                onclick={() => void copy(single)}
                aria-label={I18n.tf("Copy {single}", { single })}
            >
                <Icon name="copy" />
            </button>
        {/snippet}
    </SettingRow>
{/if}
