<script lang="ts">
    export let player: string;
</script>

<template id="player-presence-template">
    <div class="flex items-center space-x-2">
        <span class="inline-block h-2 w-2 rounded-full bg-green-500"></span>
        <span class="text-sm text-navy-100">{player}</span>
    </div>
</template>