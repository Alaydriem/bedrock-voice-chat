import { invoke } from "@tauri-apps/api/tauri";
import { WebviewWindow } from "@tauri-apps/api/window";

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
          invoke("open_webview");
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
            /*
            const webview = new WebviewWindow("microsoft_auth_login_window", {
              url: data.url,
              title: "Sign in with your Microsoft Account",
              center: true,
            });

            console.log("Call here.");
            // Once the webview starts
            webview.once("tauri://created", () => {
              console.log("window created");
              // Spawn a simple listening server to listen for the inbound request
              invoke("microsoft_auth_listener", { state: data.state })
                .then((code) => {
                  // Once we get a response back, close the window
                  webview.close();
                  // Submit the code to the API to complete the OAuth2 exchange

                  // If successful, redirect the user to the correct internal screen
                  invoke("microsoft_auth_login", {
                    server: inpt.value,
                    code: code,
                  })
                    .then((data) => data as LoginResponse)
                    .then((_data) => {
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
                  // Close the window anyways
                  webview.close();
                  inpt.classList.add("border-error");
                  errorMessage.classList.remove("invisible");
                });
            });

            console.log("Called second");
            */
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
