// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
	}
}

// Module augmentation for Svelte JSX
declare module "svelte/elements" {
	interface HTMLButtonAttributes {
		'x-tooltip.on.mouseenter'?: string;
		'x-tooltip'?: string;
		'x-data'?: any;
		'x-show'?: any;
		'x-ref'?: string;
		'@click'?: string;
		':class'?: any;
	}
	
	interface HTMLDivAttributes {
		'x-data'?: any;
		'x-show'?: any;
		'x-ref'?: string;
		'x-tooltip'?: string;
		'x-tooltip.on.mouseenter'?: string;
		'@click'?: string;
		':class'?: any;
	}

	interface SvelteHTMLElements {
		button: HTMLButtonAttributes;
		div: HTMLDivAttributes;
	}
}

export {};