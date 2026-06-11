<script lang="ts">
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        bedrockManager: BedrockManager;
    }

    let { bedrockManager }: Props = $props();

    const statusMessage = bedrockManager.statusMessage;
</script>

{#if $statusMessage}
    <div class="card p-4">
        <p class="text-sm {$statusMessage.startsWith('Error') ? 'text-error' : 'text-success'}">
            {$statusMessage}
        </p>
    </div>
{/if}
