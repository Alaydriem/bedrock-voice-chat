export default class Breakpoint {
  constructor() {
    this.#update();
    window.addEventListener("resize", this.#onResize.bind(this), {
      passive: true,
    });
  }

  height = 0;
  width = 0;
  name = "";
  isXs = false;
  isSm = false;
  isMd = false;
  isLg = false;
  isXl = false;
  is2xl = false;

  smAndDown = false;
  smAndUp = false;
  mdAndDown = false;
  mdAndUp = false;
  lgAndDown = false;
  lgAndUp = false;
  xlAndDown = false;
  xlAndUp = false;

  sm = 640;
  md = 768;
  lg = 1024;
  xl = 1280;
  the2xl = 1536;

  #firstTime = true;

  #update() {
    const height = window.innerHeight;
    const width = window.innerWidth;
    const oldName = this.name;

    const xs = width < this.sm;
    const sm = width < this.md && !xs;
    const md = width < this.lg && !(sm || xs);
    const lg = width < this.xl && !(md || sm || xs);
    const xl = width < this.the2xl && !(lg || md || sm || xs);
    const the2xl = width >= this.the2xl;

    this.height = height;
    this.width = width;

    this.isXs = xs;
    this.isSm = sm;
    this.isMd = md;
    this.isLg = lg;
    this.isXl = xl;
    this.is2xl = the2xl;

    this.smAndDown = (xs || sm) && !(md || lg || xl || the2xl);
    this.smAndUp = !xs && (sm || md || lg || xl || the2xl);
    this.mdAndDown = (xs || sm || md) && !(lg || xl || the2xl);
    this.mdAndUp = !(xs || sm) && (md || lg || xl || the2xl);
    this.lgAndDown = (xs || sm || md || lg) && !(xl || the2xl);
    this.lgAndUp = !(xs || sm || md) && (lg || xl || the2xl);
    this.xlAndDown = (xs || sm || md || lg || xl) && !the2xl;
    this.xlAndUp = !(xs || sm || md || lg) && (xl || the2xl);

    if (xs) this.name = "xs";
    if (sm) this.name = "sm";
    if (md) this.name = "md";
    if (lg) this.name = "lg";
    if (xl) this.name = "xl";
    if (the2xl) this.name = "2xl";

    document.documentElement.style.setProperty("--vh", `${height * 0.01}px`);

    if (!this.#firstTime && oldName !== this.name) {
      window.dispatchEvent(
        new CustomEvent("change:breakpoint", {
          detail: {
            height: height,
            width: this.width,
            name: this.name,
            isXs: this.isXs,
            isSm: this.isSm,
            isMd: this.isMd,
            isLg: this.isLg,
            isXl: this.isXl,
            is2xl: this.is2xl,
            smAndDown: this.smAndDown,
            smAndUp: this.smAndUp,
            mdAndDown: this.mdAndDown,
            mdAndUp: this.mdAndUp,
            lgAndDown: this.lgAndDown,
            lgAndUp: this.lgAndUp,
            xlAndDown: this.xlAndDown,
            xlAndUp: this.xlAndUp,
          },
        })
      );
    }

    this.#firstTime = false;
  }

  #onResize() {
    clearTimeout(this.resizeTimeout);
    this.resizeTimeout = window.setTimeout(this.#update.bind(this), 175);
  }
}
