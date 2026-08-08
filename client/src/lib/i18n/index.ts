// Runtime only. The build-time modules — PackCompiler, PluralExpression, PluralForms,
// PseudoLocale, SvelteSource — are deliberately absent: re-exporting them here drags the
// Svelte compiler's parser into the application bundle, because a barrel is a live import
// edge whether or not the consumer names the export.
export { CONTEXT_SEPARATOR } from "./Contract.ts";
export { default as I18n } from "./I18n.svelte.ts";
