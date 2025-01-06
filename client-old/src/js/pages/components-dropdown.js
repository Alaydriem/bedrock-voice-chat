const onLoad = () => {
  const config = {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  };

  new Popper("#dropdown-wrapper1", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper2", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper3", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper4", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper5", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper6", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper7", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper8", ".popper-ref", ".popper-root", config);
};

window.addEventListener("app:mounted", onLoad, { once: true });
