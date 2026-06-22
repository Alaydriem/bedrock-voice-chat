<script lang="ts">
    interface Step {
        title: string;
        body: string;
    }

    const steps: Step[] = [
        {
            title: "Add the BVC add-on to your Realm",
            body: "Install the no-network BVC behavior pack on the Realm you want voice on. It's a one-time setup.",
        },
        {
            title: "Everyone connects to the same BVC server",
            body: "Voice is carried by a BVC server — you and every player you want to hear must be connected to the same one.",
        },
        {
            title: "Join your Realm and talk",
            body: "Hop into the Realm in Minecraft and you'll hear everyone in proximity voice.",
        },
    ];
</script>

<div class="mt-8">
    <h3 class="text-center font-medium text-slate-700 dark:text-navy-100">How it works</h3>
    <div class="mt-4 grid grid-cols-1 gap-4 sm:grid-cols-3 sm:gap-5 lg:gap-6">
        {#each steps as step, i}
            <div class="card p-5">
                <p class="text-xs+ font-semibold uppercase tracking-wide text-primary dark:text-accent-light">
                    Step {i + 1}
                </p>
                <h4 class="mt-1 font-medium text-slate-700 dark:text-navy-100">{step.title}</h4>
                <p class="mt-1 text-sm leading-relaxed text-slate-500 dark:text-navy-300">
                    {step.body}
                </p>
            </div>
        {/each}
    </div>
</div>
