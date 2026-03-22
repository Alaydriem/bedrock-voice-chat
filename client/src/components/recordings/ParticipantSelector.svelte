<script lang="ts">
    interface Props {
        participants?: string[];
        initialSelectedParticipants?: string[];
        onSelectionChange?: (selectedParticipants: string[]) => void;
        hasJukeboxes?: boolean;
        jukeboxesEnabled?: boolean;
        onJukeboxToggle?: (enabled: boolean) => void;
        gamerpicMap?: Record<string, string>;
    }

    let {
        participants = [],
        initialSelectedParticipants = [],
        onSelectionChange = () => {},
        hasJukeboxes = false,
        jukeboxesEnabled = false,
        onJukeboxToggle = () => {},
        gamerpicMap = {},
    }: Props = $props();

    let selectedParticipants = $state(new Set<string>(
        initialSelectedParticipants.length > 0 ? initialSelectedParticipants : participants
    ));

    $effect(() => {
        onSelectionChange(Array.from(selectedParticipants));
    });

    function toggleParticipant(participant: string) {
        const newSet = new Set(selectedParticipants);
        if (newSet.has(participant)) {
            newSet.delete(participant);
        } else {
            newSet.add(participant);
        }
        selectedParticipants = newSet;
    }

    function selectAll() {
        selectedParticipants = new Set(participants);
    }

    function deselectAll() {
        selectedParticipants = new Set();
    }
</script>

<div class="space-y-3 px-4 py-3">
    <!-- Selection controls -->
    <div class="flex items-center justify-between">
        <span class="text-sm font-medium text-slate-700 dark:text-navy-100">
            Select participants to export ({selectedParticipants.size}/{participants.length})
        </span>
        <div class="flex space-x-2">
            <button
                class="btn h-7 rounded-md bg-slate-150 px-2.5 text-xs font-medium text-slate-800 hover:bg-slate-200 focus:bg-slate-200 active:bg-slate-200/80 dark:bg-navy-500 dark:text-navy-50 dark:hover:bg-navy-450 dark:focus:bg-navy-450 dark:active:bg-navy-450/90"
                onclick={selectAll}
            >
                Select All
            </button>
            <button
                class="btn h-7 rounded-md bg-slate-150 px-2.5 text-xs font-medium text-slate-800 hover:bg-slate-200 focus:bg-slate-200 active:bg-slate-200/80 dark:bg-navy-500 dark:text-navy-50 dark:hover:bg-navy-450 dark:focus:bg-navy-450 dark:active:bg-navy-450/90"
                onclick={deselectAll}
            >
                Deselect All
            </button>
        </div>
    </div>

    <!-- Participant checkboxes -->
    <div class="space-y-2">
        {#each participants as participant}
            <label class="inline-flex w-full cursor-pointer items-center space-x-2 rounded-lg px-2 py-1.5 hover:bg-slate-100 dark:hover:bg-navy-600">
                <input
                    type="checkbox"
                    checked={selectedParticipants.has(participant)}
                    onchange={() => toggleParticipant(participant)}
                    class="form-checkbox size-5 rounded border-slate-400/70 bg-slate-100 checked:bg-primary checked:border-primary checked:hover:bg-primary-focus checked:focus:bg-primary-focus dark:border-navy-400 dark:bg-navy-700 dark:checked:bg-accent dark:checked:border-accent dark:checked:hover:bg-accent-focus dark:checked:focus:bg-accent-focus"
                />
                <div class="flex items-center space-x-2">
                    <div class="avatar size-8">
                        {#if gamerpicMap[participant]}
                            <img src={gamerpicMap[participant]} alt={participant} class="mask is-squircle" />
                        {:else}
                            <div class="is-initial mask is-squircle bg-primary text-white dark:bg-accent">
                                <span class="text-sm">{participant.charAt(0).toUpperCase()}</span>
                            </div>
                        {/if}
                    </div>
                    <span class="text-sm text-slate-700 dark:text-navy-100">{participant}</span>
                </div>
            </label>
        {/each}
    </div>

    <!-- Jukeboxes toggle -->
    {#if hasJukeboxes}
        <div class="flex items-center justify-between rounded-lg border border-slate-200 px-3 py-2 dark:border-navy-500">
            <div class="flex items-center space-x-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-500 dark:text-navy-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
                </svg>
                <span class="text-sm font-medium text-slate-700 dark:text-navy-100">Jukeboxes</span>
            </div>
            <input
                type="checkbox"
                checked={jukeboxesEnabled}
                onchange={() => onJukeboxToggle(!jukeboxesEnabled)}
                class="form-switch h-5 w-10 rounded-full bg-slate-300 before:rounded-full before:bg-white checked:bg-primary checked:before:bg-white dark:bg-navy-500 dark:before:bg-navy-300 dark:checked:bg-accent dark:checked:before:bg-white"
            />
        </div>
    {/if}
</div>
