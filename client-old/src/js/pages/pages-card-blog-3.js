const onLoad = () => {
  new Popper("#top-header-menu", ".popper-ref", ".popper-root", {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  const userInfoConfig = {
    placement: "bottom",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
      { name: "preventOverflow", options: { padding: 10 } },
    ],
  };

  new Popper(
    "#userInfo1",
    ".popper-ref",
    ".popper-root",
    userInfoConfig,
    "hover"
  );

  new Popper(
    "#userInfo2",
    ".popper-ref",
    ".popper-root",
    userInfoConfig,
    "hover"
  );

  new Popper(
    "#userInfo3",
    ".popper-ref",
    ".popper-root",
    userInfoConfig,
    "hover"
  );

  new Popper(
    "#userInfo4",
    ".popper-ref",
    ".popper-root",
    userInfoConfig,
    "hover"
  );

  new Popper(
    "#userInfo5",
    ".popper-ref",
    ".popper-root",
    userInfoConfig,
    "hover"
  );

  new Popper(
    "#userInfo6",
    ".popper-ref",
    ".popper-root",
    userInfoConfig,
    "hover"
  );

  const cardMenuConfig = {
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

  new Popper("#cardMenu1", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu2", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu3", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu4", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu5", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu6", ".popper-ref", ".popper-root", cardMenuConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
