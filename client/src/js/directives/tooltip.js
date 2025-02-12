import tippy, { followCursor, roundArrow } from "tippy.js";

export default (el, { modifiers, expression }, { evaluateLater, effect }) => {
  const getContent = evaluateLater(expression);
  const options = buildOptionsFromModifiers(modifiers);

  effect(() => {
    getContent((content) => {
      if (options.content === true) {
        options.content = document
          .querySelector(content)
          .content.cloneNode(true);
        options.allowHTML = true;
        el.__x_tippy = tippy(el, options);
      } else {
        if (!el.__x_tippy) {
          el.__x_tippy = tippy(el, options);
        }
        el.__x_tippy.setContent(content);
      }
    });
  });
};

const buildOptionsFromModifiers = (modifiers) => {
  const options = {
    plugins: [],
    arrow: roundArrow,
    animation: "shift-away",
    zIndex: 10003,
  };

  if (modifiers.includes("duration")) {
    options.duration = parseInt(modifiers[modifiers.indexOf("duration") + 1]);
  }

  if (modifiers.includes("delay")) {
    options.delay = parseInt(modifiers[modifiers.indexOf("delay") + 1]);
  }

  if (modifiers.includes("cursor")) {
    options.plugins.push(followCursor);

    const next = modifiers[modifiers.indexOf("cursor") + 1] ?? null;

    if (["x", "y", "initial"].includes(next)) {
      if (next === "x") options.followCursor = "horizontal";
      if (next === "y") options.followCursor = "vertical";
      if (next === "initial") options.followCursor = "initial";
    } else {
      options.followCursor = true;
    }
  }

  if (modifiers.includes("on")) {
    options.trigger = modifiers[modifiers.indexOf("on") + 1];
  }

  if (modifiers.includes("arrowless")) {
    options.arrow = false;
  }

  if (modifiers.includes("interactive")) {
    options.interactive = true;
  }

  if (modifiers.includes("border") && options.interactive) {
    options.interactiveBorder = parseInt(
      modifiers[modifiers.indexOf("border") + 1]
    );
  }

  if (modifiers.includes("debounce") && options.interactive) {
    options.interactiveDebounce = parseInt(
      modifiers[modifiers.indexOf("debounce") + 1]
    );
  }

  if (modifiers.includes("max-width")) {
    options.maxWidth = parseInt(modifiers[modifiers.indexOf("max-width") + 1]);
  }

  if (modifiers.includes("placement")) {
    options.placement = modifiers[modifiers.indexOf("placement") + 1];
  }

  if (modifiers.includes("light")) options.theme = "light";
  if (modifiers.includes("primary")) options.theme = "primary";
  if (modifiers.includes("secondary")) options.theme = "secondary";
  if (modifiers.includes("info")) options.theme = "info";
  if (modifiers.includes("success")) options.theme = "success";
  if (modifiers.includes("warning")) options.theme = "warning";
  if (modifiers.includes("error")) options.theme = "error";
  if (modifiers.includes("sm")) options.theme = `${options.theme} is-sm`;

  if (modifiers.includes("content")) {
    options.theme = "content";
    options.content = true;
  }

  return options;
};
