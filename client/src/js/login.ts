import Main from "./main";
import { fetch } from '@tauri-apps/plugin-http';

declare global {
  interface Window {
    App: any;
  }
}

export default class Login extends Main {
  readonly CONFIG_ENDPOINT= "/api/config";
  readonly AUTH_ENDPOINT = "/api/auth";
  readonly NCRYPTF_EK_ENDPOINT = "/ncryptf/ek";

  constructor() {
    super();
  }

  async login(event: any) {
    let form = event.currentTarget;
    const serverUrl = form.querySelector("#bvc-server-input");
    const errorMessage = form.querySelector("#bvc-server-input-error-message");
    serverUrl.classList.remove("border-error");
    errorMessage.classList.add("invisible");

    await fetch(serverUrl + this.CONFIG_ENDPOINT, {
      method: 'GET'
    }).then(async (response) => {
    }).catch((e) => {
      serverUrl.classList.add("border-error");
      errorMessage.classList.remove("invisible");
    });
  }
}
