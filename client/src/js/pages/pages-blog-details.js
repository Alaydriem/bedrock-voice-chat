const onLoad = () => {

  // Profile Popover
  new Popper(
    "#popover-profile",
    ".popper-ref",
    ".popper-root",
    {
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
    },
    "hover"
  );

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

  new Popper("#blogActions", ".popper-ref", ".popper-root", dropdownConfig);

  new Popper("#cardMenu1", ".popper-ref", ".popper-root", dropdownConfig);
  new Popper("#cardMenu2", ".popper-ref", ".popper-root", dropdownConfig);
  new Popper("#cardMenu3", ".popper-ref", ".popper-root", dropdownConfig);
  new Popper("#cardMenu4", ".popper-ref", ".popper-root", dropdownConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
