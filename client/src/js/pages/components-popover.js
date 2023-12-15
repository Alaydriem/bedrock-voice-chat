const onLoad = () => {
  const config1 = {
    placement: "right-start",
    modifiers: [
      { name: "flip", options: { fallbackPlacements: ["bottom", "top"] } },
      { name: "preventOverflow", options: { padding: 10 } },
      {
        name: "offset",
        options: {
          offset: [0, 12],
        },
      },
    ],
  };

  // Basic popover
  new Popper("#popover-wrapper1", ".popper-ref", ".popper-root", config1);

  // Advanced popover
  new Popper("#popover-wrapper2", ".popper-ref", ".popper-root", config1);

  // Form popover
  new Popper("#popover-wrapper3", ".popper-ref", ".popper-root", config1);

  const config2 = {
    placement: "top",
    modifiers: [
      { name: "preventOverflow", options: { padding: 10 } },
      {
        name: "offset",
        options: {
          offset: [0, 12],
        },
      },
    ],
  };

  // Profile Popover 1
  new Popper(
    "#popover-wrapper4",
    ".popper-ref",
    ".popper-root",
    config2,
    "hover"
  );

  new Popper(
    "#popover-wrapper5",
    ".popper-ref",
    ".popper-root",
    config2,
    "hover"
  );

  new Popper(
    "#popover-wrapper6",
    ".popper-ref",
    ".popper-root",
    config2,
    "hover"
  );

  // Profile Popover 2
  new Popper(
    "#popover-wrapper7",
    ".popper-ref",
    ".popper-root",
    config2,
    "hover"
  );

  new Popper(
    "#popover-wrapper8",
    ".popper-ref",
    ".popper-root",
    config2,
    "hover"
  );

  new Popper(
    "#popover-wrapper9",
    ".popper-ref",
    ".popper-root",
    config2,
    "hover"
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
