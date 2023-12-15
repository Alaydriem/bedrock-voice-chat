const onLoad = () => {
  const config = {
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

  new Popper("#dropdown-wrapper1", ".popper-ref", ".popper-root", config);
  new Popper("#dropdown-wrapper2", ".popper-ref", ".popper-root", config);

  new Drawer('#drawer1')
  new Drawer('#drawer2')
};

window.addEventListener("app:mounted", onLoad, { once: true });
