const MONOCHROMEMODE_CLASS = "is-monochrome";

export default class MonochromeMode {
  currentMode = "";

  constructor(initialVal = "") {
    if (initialVal === "monochrome") {
      document.body.classList.add(MONOCHROMEMODE_CLASS);
      this.currentMode = "monochrome";
    }
  }

  setMonochrome() {
    document.body.classList.add(MONOCHROMEMODE_CLASS);
    this.currentMode = "monochrome";
    window.dispatchEvent(
      new CustomEvent("change:monochrome", {
        detail: { currentMode: "monochrome" },
      })
    );
  }

  removeMonochrome() {
    document.body.classList.remove(MONOCHROMEMODE_CLASS);
    this.currentMode = "";
    window.dispatchEvent(
      new CustomEvent("change:monochrome", {
        detail: { currentMode: "" },
      })
    );
  }

  toggle() {
    this.currentMode === "monochrome"
      ? this.removeMonochrome()
      : this.setMonochrome();
  }
}
