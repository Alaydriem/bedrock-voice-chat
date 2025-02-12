import { createPopper } from "@popperjs/core";

export default (userOptions = {}) => ({
  popperInstance: null,
  options: buildOptions(userOptions),
  isShowPopper: false,
  init() {
    this.$nextTick(
      () =>
      (this.popperInstance = createPopper(
        this.$refs.popperRef,
        this.$refs.popperRoot,
        this.options
      ))
    );
    this.$watch("isShowPopper", (val) => {
      val && this.popperInstance.update()
    });

    window.addEventListener('changed:breakpoint', hidePopper.bind(this))
  },

  destroy() {
    this.popperInstance.destroy();
    window.removeEventListener('changed:breakpoint', hidePopper.bind(this))
    this.popperInstance = null;
    this.isShowPopper = false;
  },
});

function hidePopper() {
  this.isShowPopper = false
}

const buildOptions = (options) => {
  const config = {
    placement: options.placement ?? "auto",
    strategy: options.strategy ?? "fixed",
    onFirstUpdate: options.onFirstUpdate ?? function () { },

    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, options.offset ?? 0],
        },
      },
    ],
  };

  if (options.modifiers) config.modifiers.push(...options.modifiers);

  return config;
};
