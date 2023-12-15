const onLoad = () => {
  //Sidebar channels collapse
  new Accordion("#sidebar-channels", {
    duration: 200,
    openOnInit: [0],
  });

  // Food Category Slider
  const categoriesSlide = document.querySelector("#categories");
  categoriesSlide._swiper = new Swiper(categoriesSlide, {
    slidesPerView: "auto",
    spaceBetween: 14,
    navigation: { nextEl: ".next-btn", prevEl: ".prev-btn" },
  });

  // Cart Drafts Menu
  new Popper("#cart-drafts-menu", ".popper-ref", ".popper-root", {
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

  // Cart Menu
  new Popper("#cart-menu", ".popper-ref", ".popper-root", {
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

  const drawer = new Modal("#posSheet");

  window.addEventListener('change:breakpoint', (e) => {
    if (e.detail.smAndUp) {
      drawer.close()
    }
  })
};

window.addEventListener("app:mounted", onLoad, { once: true });
