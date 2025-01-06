const onLoad = () => {
  // Sidebar Lables Menu
  new Popper("#sidebar-labels-menu", ".popper-ref", ".popper-root", {
    placement: "bottom-end",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  // Top Sidebar Menu
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
};

window.addEventListener("app:mounted", onLoad, { once: true });
