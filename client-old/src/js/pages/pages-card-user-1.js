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
  new Popper("#cardMenu7", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu8", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu9", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu10", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu11", ".popper-ref", ".popper-root", cardMenuConfig);
  new Popper("#cardMenu12", ".popper-ref", ".popper-root", cardMenuConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
