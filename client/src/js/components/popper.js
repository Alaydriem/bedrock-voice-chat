import { createPopper } from "@popperjs/core";

const POPPER_SHOW_CLASS = "show";
const REF_ACTIVE_CLASS = "is-active";

export default class Popper {
  isShowPopper = false;

  constructor(wrapper, ref, box, options, evt = "click", onToggle = () => { }) {
    if (typeof wrapper == 'string') {
      this.wrapperEl = document.querySelector(wrapper);
    } else {
      this.wrapperEl = wrapper;
    }

    this.ref = this.wrapperEl.querySelector(ref);
    this.box = this.wrapperEl.querySelector(box);

    this.onToggle = onToggle;
    this.wrapperEl._popper = this
    /**
     *
     * Create Popper instance
     */
    this.instance = createPopper(this.ref, this.box, options);

    if (evt === "hover") {
      this.wrapperEl.addEventListener("mouseenter", () => this.showPopper());
      this.wrapperEl.addEventListener("mouseleave", () => this.closePopper());
      return;
    }

    this.ref.addEventListener(evt, () => this.togglePopper());

    /**
     *
     * Close Popper when click outside
     */
    document.addEventListener(
      "click",
      (event) => {
        if (!this.wrapperEl.contains(event.target)) {
          if (this.isShowPopper) this.closePopper();
        }
      },
      false
    );

    /**
     *
     * Close Popper when breakpoint changed
     */
    window.addEventListener("change:breakpoint", () => {
      if (this.isShowPopper) this.closePopper();
    });
  }

  showPopper() {
    this.instance.update()
    this.box.classList.add(POPPER_SHOW_CLASS);
    this.ref.classList.add(REF_ACTIVE_CLASS);
    this.isShowPopper = true;
    this.onToggle(this.isShowPopper);
  }

  closePopper() {
    this.box.classList.remove(POPPER_SHOW_CLASS);
    this.ref.classList.remove(REF_ACTIVE_CLASS);
    this.isShowPopper = false;
    this.onToggle(this.isShowPopper);
  }

  togglePopper() {
    if (!this.isShowPopper) {
      this.showPopper();
    } else {
      this.closePopper();
    }
  }
}
