<script lang="ts">
    import "../css/app.css";
    import Splash from "../js/app/splash.ts";

    import { onMount } from 'svelte';

    onMount(async () => {
        window.App = new Splash();
        await window.App.initialize();
        window.dispatchEvent(new CustomEvent("app:mounted"));
    });
</script>


<div id="root" class="min-h-80vh cloak flex grow bg-slate-50 dark:bg-navy-900">
    <main class="grid w-full grow grid-cols-1 place-items-center">
    <div class="w-full max-w-[26rem] p-4 sm:px-5">
        <div class="text-center">
        <img
            class="mx-auto h-32 w-32"
            src="/images/app-logo.png"
            alt="Bedrock Voice Chat Logo"
        />
        <div class="mt-4">
            <h2 class="text-2xl font-semibold text-slate-600 dark:text-navy-100">
            Bedrock Voice Chat
            </h2>
            <p class="mt-4">Checking for updates and other stuff...</p>
        </div>
        </div>
    </div>
    </main>
</div>