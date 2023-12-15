import Main from "./main";
import Login from "./login";

declare global {
  interface Window {
    App: any;
  }
}

export default class Application extends Main {
  constructor() {
    super();
    // Your Application

    const login = new Login();
    console.log("load");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  window.App = new Application();
  window.dispatchEvent(new CustomEvent("app:mounted"));
});
