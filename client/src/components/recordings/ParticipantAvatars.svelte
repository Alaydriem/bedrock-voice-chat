<script lang="ts">
    interface Props {
        participants?: string[];
        gamerpicMap?: Record<string, string>;
        maxVisible?: number;
    }

    let { participants = [], gamerpicMap = {}, maxVisible = 4 }: Props = $props();

    function getInitials(name: string): string {
        return name.charAt(0).toUpperCase();
    }

    function getColorClass(_name: string, index: number): string {
        const colors = [
            'bg-primary text-white',
            'bg-secondary text-white',
            'bg-info text-white',
            'bg-success text-white',
            'bg-warning text-white',
            'bg-error text-white'
        ];
        return colors[index % colors.length];
    }

    let visibleParticipants = $derived(participants.slice(0, maxVisible));
    let remainingCount = $derived(Math.max(0, participants.length - maxVisible));
</script>

<div class="flex flex-wrap -space-x-2">
    {#each visibleParticipants as participant, index}
        <div class="avatar size-8 hover:z-10">
            {#if gamerpicMap[participant]}
                <img src={gamerpicMap[participant]} alt={participant} class="rounded-full ring-2 ring-white dark:ring-navy-700" />
            {:else}
                <div class="is-initial rounded-full text-xs-plus uppercase ring-2 ring-white dark:ring-navy-700 {getColorClass(participant, index)}">
                    {getInitials(participant)}
                </div>
            {/if}
        </div>
    {/each}

    {#if remainingCount > 0}
        <div class="avatar size-8 hover:z-10">
            <div class="is-initial rounded-full bg-slate-400 text-xs-plus text-white ring-2 ring-white dark:ring-navy-700 dark:bg-navy-400">
                +{remainingCount}
            </div>
        </div>
    {/if}
</div>
