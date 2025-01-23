import Main from "./main";
import { fetch } from '@tauri-apps/plugin-http';
import { platform } from '@tauri-apps/plugin-os';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { openPath, openUrl } from '@tauri-apps/plugin-opener';

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

    await fetch(serverUrl.value + this.CONFIG_ENDPOINT, {
      method: 'GET'
    }).then(async (response) => {
      if (response.status !== 200) {
        throw new Error("Bedrock Voice Chat Server " + serverUrl.value + " is not reachable.");
      }
      return response.json();
    }).then(async (response) => {
      const clientId = response.client_id;
      const secretState = self.crypto.randomUUID();
      const store = await Store.load('store.json', { autoSave: false });
      await store.set("auth_state_token", { value: secretState });
      await store.save();

      const androidSignatureHash = await store.get<{ value: string }>("android_signature_hash");

      const redirectUrl = (() => {
        switch (platform()) {
          case "windows": return "bedrock-voice-chat://auth";
          case "android": return "mmsauth://com.alaydriem.bvc/" + androidSignatureHash?.value;
          case "ios": return "msauth.com.alaydriem.bvc.client://auth";
          default: throw new Error("Unsupported platform");
        };
      })();

      const authLoginUrl: string = 
        `https://login.live.com/oauth20_authorize.srf?client_id=${clientId}&response_type=code&redirect_uri=${redirectUrl}&scope=XboxLive.signin%20offline_access&state=${secretState}`;

      openUrl(authLoginUrl);
    }).catch((e) => {
      warn(e);
      serverUrl.classList.add("border-error");
      errorMessage.classList.remove("invisible");
    });
  }

  async openDeepLink(urls: string[]) {
    info(JSON.stringify(urls));
  }
}
