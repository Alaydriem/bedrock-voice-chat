<script lang="ts">
    interface Props {
        featured?: boolean;
    }

    let { featured = false }: Props = $props();
</script>

<div class="card h-full animate-pulse overflow-hidden rounded-2xl {featured ? 'ring-2 ring-slate-200 dark:ring-navy-600' : ''}">
    <div class="rounded-t-2xl p-5 text-center {featured ? 'bg-slate-200 dark:bg-navy-700' : 'bg-slate-150 dark:bg-navy-800'}">
        <div class="mx-auto h-4 w-20 rounded bg-slate-300/70 dark:bg-navy-600"></div>
        <div class="mx-auto mt-3 h-9 w-28 rounded bg-slate-300/70 dark:bg-navy-600"></div>
        <div class="mx-auto mt-3 h-3 w-16 rounded bg-slate-300/70 dark:bg-navy-600"></div>
    </div>
    <div class="p-5">
        <div class="mx-auto h-3 w-40 rounded bg-slate-200 dark:bg-navy-600"></div>
        <div class="mt-5 h-9 w-full rounded-full bg-slate-200 dark:bg-navy-600"></div>
    </div>
</div>
