/**
 * Tooltip Library
 * @see https://atomiks.github.io/tippyjs/
 */
import tippy, { followCursor, roundArrow } from "tippy.js";

const tippyPlugins = { followCursor, roundArrow };

const TOOLTIP_SELECTOR = "[data-tooltip]";

const InitTooltips = () => {
  const tooltips = document.querySelectorAll(TOOLTIP_SELECTOR);

  if (!tooltips) return;

  const options = (data) => {
    const config = {
      plugins: [],
      content: data.tooltip,
      arrow: tippyPlugins.roundArrow,
      animation: "shift-away",
      zIndex: 10003,
    };

    if (data.placement) config.placement = data.placement;
    if (data.tooltipTheme) config.theme = data.tooltipTheme;
    if (data.tooltipDelay) config.delay = parseInt(data.tooltipDelay);
    if (data.tooltipDuration) config.duration = parseInt(data.tooltipDuration);
    if (data.tooltipTrigger) config.trigger = data.tooltipTrigger;

    if (data.tooltipFollowCursor !== undefined) {
      config.plugins.push(followCursor);

      if (["x", "y", "initial"].includes(data.tooltipFollowCursor)) {
        if (data.tooltipFollowCursor === "x")
          config.followCursor = "horizontal";
        if (data.tooltipFollowCursor === "y") config.followCursor = "vertical";
        if (data.tooltipFollowCursor === "initial")
          config.followCursor = "initial";
      } else {
        config.followCursor = true;
      }
    }

    if (data.contentHtml !== undefined) {
      config.content = document
        .querySelector(data.tooltip)
        .content.cloneNode(true);
      config.allowHTML = true;
      config.interactive = true;
      config.theme = "content";
    }

    return config;
  };

  tooltips.forEach((node) => {
    tippy(node, options(node.dataset));
  });
};

window.addEventListener("app:mounted", InitTooltips, { once: true });

export { tippy, tippyPlugins };