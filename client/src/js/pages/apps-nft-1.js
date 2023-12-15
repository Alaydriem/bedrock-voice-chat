const onLoad = () => {
  // Balance Menu
  new Popper("#balance-menu", ".popper-ref", ".popper-root", {
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

  // Featured Author Menu
  new Popper("#featured-author-menu", ".popper-ref", ".popper-root", {
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

  // Top NFT table tabs
  new Tab("#top-nft-tab");
};

window.addEventListener("app:mounted", onLoad, { once: true });
