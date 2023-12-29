import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/shell";
type MicrosoftAuthCodeAndUrlResponse = {
  url: string;
  state: string;
};

type ApiConfig = {
  status: string;
  client_id: string;
};

type LoginResponse = {
  key: string;
  cert: string;
  gamerpic: string;
  gamertag: string;
};

export default class Login {
  form: any;

  constructor() {
    const page = document.querySelector("#login-page");

    if (page != null) {
      // If we have credentials for a user, push them to the dashboard
      invoke("get_credential", { key: "cert" })
        .then((_result) => {
          window.location.href = "dashboard.html";
        })
        .catch(() => {
          // Make the main page visible
          page?.querySelector("#root")?.classList.remove("invisible");
          this.form = document.querySelector("#login-form");
          this.form.form = this.form;

          this.form.addEventListener("submit", this.submitLoginForm);
        });
    }
  }

  async submitLoginForm(event: any) {
    let form = event.currentTarget.form;
    event.preventDefault();

    // Reset the input form state
    const inpt = form.querySelector("#bvc-server-input");
    const errorMessage = form.querySelector("#bvc-server-input-error-message");
    inpt.classList.remove("border-error");
    errorMessage.classList.add("invisible");

    // Ping the API to ensure that we have connectivity to the selected BVC endpoint
    invoke("check_api_status", { server: inpt.value })
      .then(async (data) => data as ApiConfig)
      .then(async (data) => {
        invoke("microsoft_auth", { cid: data.client_id })
          .then(async (data) => data as MicrosoftAuthCodeAndUrlResponse)
          .then(async (data) => {
            // Spawn a simple listening server to listen for the inbound request
            invoke("microsoft_auth_listener", { state: data.state })
              .then((code) => {
                // Submit the code to the API to complete the OAuth2 exchange
                console.log(code);
                // If successful, redirect the user to the correct internal screen
                invoke("microsoft_auth_login", {
                  server: inpt.value,
                  code: code,
                })
                  .then((data) => data as LoginResponse)
                  .then((data) => {
                    console.log(data);
                    // We can pull data from keychain as necessary
                    window.location.href = "dashboard.html";
                  })
                  .catch((error) => {
                    console.log(error);
                    inpt.classList.add("border-error");
                    errorMessage.classList.remove("invisible");
                  });
              })
              .catch((_error) => {
                inpt.classList.add("border-error");
                errorMessage.classList.remove("invisible");
              });

            // Tauri WebviewWindow doesn't work
            await open(data.url);
          })
          .catch((_error) => {
            inpt.classList.add("border-error");
            errorMessage.classList.remove("invisible");
          });
      })
      .catch((_error) => {
        inpt.classList.add("border-error");
        errorMessage.classList.remove("invisible");
      });
  }
}
