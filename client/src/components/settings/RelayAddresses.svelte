<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import type { RelayAddress } from "../../js/app/settings/BedrockRelayAddresses";

    interface Props {
        addresses: readonly RelayAddress[];
    }
    let { addresses }: Props = $props();

    async function copy(text: string): Promise<void> {
        await navigator.clipboard?.writeText(text).catch(() => {});
    }
</script>

{#if addresses.length > 0}
    <SettingRow
        label="Or go through this BVC server"
        note="Reaches the same session without this device having to stay on the network you play from."
        stack
    >
        <div class="rad-addresses">
            {#each addresses as offer (offer.address)}
                <div class="rad-address rad-address--offer">
                    <span class="rad-address__value">{offer.address}</span>
                    <span class="rad-address__label">{offer.label}</span>
                    <button
                        class="rad-icon-btn"
                        onclick={() => void copy(offer.address)}
                        aria-label="Copy {offer.address}"
                    >
                        <Icon name="copy" />
                    </button>
                    <span class="rad-address__note">{offer.note}</span>
                </div>
            {/each}
        </div>
    </SettingRow>
{/if}
