<script lang="ts">
    import { onMount } from "svelte";

    export let id: string = "knob";
    export let src: string = "../../../../node_modules/webaudio-controls/knobs/LittlePhatty.png";
    export let value: number = 0;
    export let min: number = 0;
    export let max: number = 100;
    export let step: number = 1;
    export let label: string = "Knob";
    export let diameter: number = 64;
    export let sprites: number = 100;

    onMount(() => {
        const knob = document.querySelector("webaudio-knob#" + id);
        knob?.addEventListener("change", (e: Event) => {
            const target = e.target as HTMLInputElement;
            value = parseFloat(target.value);
            console.log(`Knob ${id} changed to: ${value}`);
        });
    });
</script>

<webaudio-knob
    class="inline-flex"
    id={id}
    valuetip=1
    src={src}
    value={value}
    min={min}
    max={max}
    step={step}
    label={label}
    diameter={diameter}
    sprites={sprites}
>{label}</webaudio-knob>