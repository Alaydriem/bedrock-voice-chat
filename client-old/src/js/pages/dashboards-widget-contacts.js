const onLoad = () => {
  // Contact List Accordion
  new Accordion("#contact-list-accordion", {
    duration: 200,
    openOnInit: [0],
  });

  // Dropdown Menu Config
  const dropdownConfig = {
    placement: "bottom-end",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  };

  // Contact List Menu
  new Popper(
    "#contact-list-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Contact Info Menu
  new Popper(
    "#contact-info-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Call Setting Menu
  new Popper(
    "#call-setting-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
