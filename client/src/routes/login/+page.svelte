<script lang="ts">
  import "../../css/app.css";
  import Login from "../../js/app/login.ts";
  import { onMount } from 'svelte';

  onMount(async () => {
    window.App = new Login();
    window.dispatchEvent(new CustomEvent("app:mounted"));

    // Initialize and check for pending deep link callbacks
    await window.App.initialize();

    window.App.preloader();

    const urlParams = new URLSearchParams(window.location.search);

    document.querySelector("#login-form")
      ?.addEventListener("submit", (e) => {
        window.App.login(e);
    });

    // Auto-format the server URL when the input loses focus
    const serverInput = document.querySelector("#bvc-server-input") as HTMLInputElement;
    serverInput?.addEventListener("blur", () => {
      if (serverInput.value.trim()) {
        let formatted = serverInput.value.trim();

        // Add https:// prefix if not present
        if (!formatted.startsWith("https://") && !formatted.startsWith("http://")) {
          formatted = "https://" + formatted;
        } else if (formatted.startsWith("http://")) {
          formatted = formatted.replace("http://", "https://");
        }

        // Remove trailing slash
        if (formatted.endsWith("/")) {
          formatted = formatted.slice(0, -1);
        }

        serverInput.value = formatted;
      }
    });

    if (urlParams.has("server")) {
      const server = urlParams.get("server") ?? "";
      document.querySelector("#bvc-server-input")?.setAttribute("value", server);

      if (urlParams.has("reauth")) {
        const reauth = urlParams.get("reauth");
        if (reauth === "true") {
          document.querySelector("#login-form")
            ?.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
        }
      }
    }
  });
</script>

<div
      id="root"
      class="min-h-80vh cloak flex grow bg-slate-50 dark:bg-navy-900"
    >
      <main class="grid w-full grow grid-cols-1 place-items-center">
        <div class="w-full max-w-[26rem] p-4 sm:px-5">
          <div class="text-center">
            <img
              class="mx-auto h-32 w-32"
              src="/images/app-logo-transparent.png"
              alt="Bedrock Voice Chat Logo"
            />
            <div class="mt-4">
              <h2
                class="text-2xl font-semibold text-slate-600 dark:text-navy-100"
              >
                Bedrock Voice Chat
              </h2>
            </div>
          </div>
          <form id="login-form">
            <div class="card mt-5 rounded-lg p-5 lg:p-7">
              <div>
                <p class="text-slate-300 dark:text-navy-200 pb-3">
                  Set your server, then sign in with your account
                </p>
                <label class="mt-1.5 flex -space-x-px">
                  <span
                    class="flex items-center justify-center rounded-l-lg border border-slate-300 px-3.5 font-inter dark:border-navy-450"
                  >
                    <span class="-mt-1"
                      ><i class="fa-solid fa-server" style="color: #ffffff"></i
                    ></span>
                  </span>
                  <input
                    id="bvc-server-input"
                    class="form-input w-full rounded-r-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 hover:z-10 hover:border-slate-400 focus:z-10 focus:border-primary dark:border-navy-450 dark:hover:border-navy-400 dark:focus:border-accent"
                    placeholder="bvc.alaydriem.com"
                    type="url"
                    autocorrect="off"
                    autocapitalize="none"
                    spellcheck="false"
                    autocomplete="url"
                  />
                </label>
                <span
                  id="bvc-server-input-error-message"
                  class="text-tiny+ text-error invisible"
                  >Unable to connect and verify BVC Server. Check the URL?</span
                >
              </div>
              <button class="btn mt-5 w-full">
                <img
                  src="/images/ms-symbollockup_signin_dark.svg"
                  alt="Sign in with Microsoft Account"
                  width="215"
                  height="41"
                />
              </button>
              <button id="hytale-login-btn" type="button" class="btn mt-3 w-full" on:click={(e) => window.App.loginWithHytale(e)}>
                <img
                  src="/images/hytale-login-button.svg"
                  alt="Sign in with Hytale"
                  width="215"
                  height="41"
                />
              </button>
            </div>
          </form>
          <div
            class="mt-8 flex justify-center text-xs text-slate-400 dark:text-navy-300"
          >
            <a href="https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md">Privacy Notice</a>
            <div class="mx-3 my-1 w-px bg-slate-200 dark:bg-navy-500"></div>
          </div>
        </div>
      </main>
    </div>