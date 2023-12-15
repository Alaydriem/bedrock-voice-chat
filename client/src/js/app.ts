import { invoke } from "@tauri-apps/api/tauri";
import Main from "./main";

declare global {
  interface Window {
    App: any;
  }
}

export default class Application extends Main {
  constructor() {
    super();
    // Your Application

    console.log("load");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  window.App = new Application();
  window.dispatchEvent(new CustomEvent("app:mounted"));
});
