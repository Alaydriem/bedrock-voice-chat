const DRAWER_OVERLAY_CLASS = "drawer-overlay";
const DRAWER_CONTENT_CLASS = "drawer-content";
const DRAWER_CLOSE_SELECTOR = "[data-close-drawer]";

export default class Drawer {
  isActive = false;

  constructor(selector, onToggle = () => { }) {
    if (!selector) {
      throw new TypeError("Error: Drawer not defined");
    }

    this.drawer = document.querySelector(selector);
    this.selector = selector;

    this.onToggle = onToggle;

    this.overlay = this.drawer.querySelector(`.${DRAWER_OVERLAY_CLASS}`);
    this.content = this.drawer.querySelector(`.${DRAWER_CONTENT_CLASS}`);

    if (!this.content) {
      throw new TypeError("Error: Drawer content not defined");
    }

    const toggleBtns = document.querySelectorAll(
      `[data-toggle="drawer"][data-target="${selector}"]`
    );

    const closeBtns = this.drawer.querySelectorAll(DRAWER_CLOSE_SELECTOR);

    toggleBtns.forEach((node) => {
      node.addEventListener("click", () => this.toggle());
    });

    closeBtns.forEach((node) => {
      node.addEventListener("click", () => this.close());
    });

    if (this.overlay)
      this.overlay.addEventListener("click", () => this.close());
  }

  open() {
    if (this.isActive) return;

    if (this.overlay) this.overlay.classList.remove("hidden");
    this.content.classList.remove("hidden");

    this.isActive = true;

    this.onToggle(this.isActive);
  }

  close() {
    if (!this.isActive) return;

    if (this.overlay) {
      helpers.leaveAnimation(this.overlay, () => {
        this.overlay.classList.add("hidden");
      });
    }

    helpers.leaveAnimation(this.content, () => {
      this.content.classList.add("hidden");
    });

    this.isActive = false;

    this.onToggle(this.isActive);
  }

  toggle() {
    this.isActive ? this.close() : this.open();
  }
}
