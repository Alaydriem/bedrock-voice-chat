import Main from "./main";

declare global {
  interface Window {
    App: any;
  }
}

export default class Login extends Main {
  constructor() {
    super();
  }
}
