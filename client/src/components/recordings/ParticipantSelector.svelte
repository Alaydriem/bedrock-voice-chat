<script lang="ts">
    export let participants: string[] = [];
    export let initialSelectedParticipants: string[] = [];
    export let onSelectionChange: (selectedParticipants: string[]) => void = () => {};

    // Initialize with provided selected participants or all participants if not provided
    let selectedParticipants = new Set<string>(
        initialSelectedParticipants.length > 0 ? initialSelectedParticipants : participants
    );

    // Reactive statement to notify parent when selection changes
    $: {
        onSelectionChange(Array.from(selectedParticipants));
    }

    function toggleParticipant(participant: string) {
        const newSet = new Set(selectedParticipants);
        if (newSet.has(participant)) {
            newSet.delete(participant);
        } else {
            newSet.add(participant);
        }
        selectedParticipants = newSet; // Trigger reactivity
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
                on:click={selectAll}
            >
                Select All
            </button>
            <button
                class="btn h-7 rounded-md bg-slate-150 px-2.5 text-xs font-medium text-slate-800 hover:bg-slate-200 focus:bg-slate-200 active:bg-slate-200/80 dark:bg-navy-500 dark:text-navy-50 dark:hover:bg-navy-450 dark:focus:bg-navy-450 dark:active:bg-navy-450/90"
                on:click={deselectAll}
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
                    on:change={() => toggleParticipant(participant)}
                    class="form-checkbox size-5 rounded border-slate-400/70 bg-slate-100 checked:bg-primary checked:border-primary checked:hover:bg-primary-focus checked:focus:bg-primary-focus dark:border-navy-400 dark:bg-navy-700 dark:checked:bg-accent dark:checked:border-accent dark:checked:hover:bg-accent-focus dark:checked:focus:bg-accent-focus"
                />
                <div class="flex items-center space-x-2">
                    <div class="avatar size-8">
                        <div class="is-initial mask is-squircle bg-primary text-white dark:bg-accent">
                            <span class="text-sm">{participant.charAt(0).toUpperCase()}</span>
                        </div>
                    </div>
                    <span class="text-sm text-slate-700 dark:text-navy-100">{participant}</span>
                </div>
            </label>
        {/each}
    </div>
</div>
