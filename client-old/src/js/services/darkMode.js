const DARKMODE_KEY = "dark-mode";
const DARKMODE_VALUE = "dark";
const DARKMODE_CLASS = "dark";

export default class DarkMode {
  currentMode = "";

  constructor(initialVal = "") {
    const initial =
      localStorage.getItem(DARKMODE_KEY) !== null
        ? localStorage.getItem(DARKMODE_KEY)
        : initialVal;

    localStorage.setItem(DARKMODE_KEY, initial);

    if (initial === DARKMODE_VALUE) {
      document.documentElement.classList.add(DARKMODE_CLASS);
      this.currentMode = "dark";
    } else {
      this.currentMode = "light";
    }
  }

  setDarkMode() {
    localStorage.setItem(DARKMODE_KEY, DARKMODE_VALUE);
    document.documentElement.classList.add(DARKMODE_CLASS);
    this.currentMode = "dark";
    window.dispatchEvent(
      new CustomEvent("change:darkmode", {
        detail: { currentMode: "dark" },
      })
    );
  }

  setLightMode() {
    localStorage.setItem(DARKMODE_KEY, "");
    document.documentElement.classList.remove(DARKMODE_CLASS);
    this.currentMode = "light";
    window.dispatchEvent(
      new CustomEvent("change:darkmode", {
        detail: { currentMode: "light" },
      })
    );
  }

  toggle() {
    this.currentMode === "light" ? this.setDarkMode() : this.setLightMode();
  }
}
