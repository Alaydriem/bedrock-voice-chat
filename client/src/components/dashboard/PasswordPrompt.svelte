<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";

    interface Props {
        /** What the password is for: creating a group, or joining one. */
        title: string;
        onsubmit: (password: string) => void;
        oncancel: () => void;
    }
    let { title, onsubmit, oncancel }: Props = $props();

    // Matches the app's own check, so the common mistake is caught before a round trip.
    const MIN_CHARS = 6;

    let password = $state("");
    let tooShort = $state(false);

    function submit(event: SubmitEvent): void {
        event.preventDefault();
        // Counted in code points after NFC, as the app counts them.
        if ([...password.normalize("NFC")].length < MIN_CHARS) {
            tooShort = true;
            return;
        }
        const value = password;
        password = "";
        onsubmit(value);
    }
</script>

<form class="rad-password-prompt" onsubmit={submit}>
    <label>
        <span>{title}</span>
        <input
            class="rad-password-prompt__input"
            type="password"
            autocomplete="off"
            bind:value={password}
            oninput={() => (tooShort = false)}
        />
    </label>
    {#if tooShort}
        <span class="rad-password-prompt__error" role="alert">
            {I18n.t("A group password needs at least 6 characters.")}
        </span>
    {/if}
    <div class="rad-password-prompt__actions">
        <button type="button" class="rad-new-group" onclick={oncancel}>{I18n.t("Cancel")}</button>
        <button type="submit" class="rad-new-group">{I18n.t("Continue")}</button>
    </div>
</form>
