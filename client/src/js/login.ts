import { invoke } from "@tauri-apps/api/tauri";
import { WebviewWindow } from '@tauri-apps/api/window'

type MicrosoftAuthCodeAndUrlResponse = {
    url: string,
    state: string
};

type ApiConfig = {
    status: string,
    client_id: string
};

type LoginResponse = {
    key: string,
    cert: string,
    gamerpic: string,
    gamertag: string
};

export default class Login {
    form: any;

    constructor() {
        this.form = document.querySelector("#login-form");
        this.form.addEventListener("submit", this.submitLoginForm);
        this.form.form = this.form;
    }

    submitLoginForm(event: any) {
        let form = event.currentTarget.form;
        event.preventDefault();

        // Reset the input form state
        const inpt = form.querySelector("#bvc-server-input");
        const errorMessage = form.querySelector("#bvc-server-input-error-message");
        inpt.classList.remove("border-error");
        errorMessage.classList.add("invisible");

        // Ping the API to ensure that we have connectivity to the selected BVC endpoint
        invoke("check_api_status", { server: inpt.value })
            .then((data) => data as ApiConfig)
            .then((data) => {
                console.log(data);
                invoke("microsoft_auth", { cid: data.client_id })
                    .then((data) => data as MicrosoftAuthCodeAndUrlResponse)
                    .then((data) => {
                        console.log("We hit here now?");
                        // Open the webview
                        const webview = new WebviewWindow('microsoft_auth_login_window', {
                            url: data.url
                        });

                        // Once the webview starts
                        webview.once('tauri://created', () => {
                            // Spawn a simple listening server to listen for the inbound request
                            invoke("microsoft_auth_listener", { state: data.state }).then((code) => {
                                // Once we get a response back, close the window
                                webview.close();
                                // Submit the code to the API to complete the OAuth2 exchange

                                // If successful, redirect the user to the correct internal screen
                                invoke("microsoft_auth_login", { server: inpt.value, code: code })
                                    .then((data) => data as LoginResponse)
                                    .then((data) => {
                                        console.log(data);
                                        console.log("We hit the login endpoint and did the thing!");
                                    }).catch((error) => {
                                        console.log(error);
                                        inpt.classList.add("border-error");
                                        errorMessage.classList.remove("invisible");
                                    })
                            }).catch((error) => {
                                console.log(error);
                                // Close the window anyways
                                webview.close();
                                inpt.classList.add("border-error");
                                errorMessage.classList.remove("invisible");
                            });
                        });
                    }).catch((error) => {
                        console.log(error);
                        inpt.classList.add("border-error");
                        errorMessage.classList.remove("invisible");
                    });
            }).catch((error) => {
                console.log(error);
                inpt.classList.add("border-error");
                errorMessage.classList.remove("invisible");
            });
    }
}