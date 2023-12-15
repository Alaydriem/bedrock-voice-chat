const onLoad = () => {
  const config1 = {
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
  };

  const config2 = {
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
    pagination: { el: ".swiper-pagination", type: "progressbar" },
    lazy: true,
  };

  const config3 = {
    slidesPerView: "auto",
    spaceBetween: 30,
    pagination: { el: ".swiper-pagination", clickable: true },
  };

  const config4 = {
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
    pagination: { el: ".swiper-pagination", clickable: true },
    zoom: { maxRatio: 4 },
  };

  const config5 = {
    effect: "flip",
    flipEffect: { slideShadows: false },
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
    pagination: { el: ".swiper-pagination", clickable: true },
  };

  const config6 = {
    effect: "cube",
    cubeEffect: { shadow: false },
    pagination: { el: ".swiper-pagination", clickable: true },
  };

  const config7 = { effect: "cards", grabCursor: true };

  const config8 = { pagination: { el: ".swiper-pagination", clickable: true } };

  const config9 = {
    direction: "vertical",
    pagination: { el: ".swiper-pagination", clickable: true },
  };

  const config10 = {
    scrollbar: { el: ".swiper-scrollbar", draggable: true },
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
    autoplay: { delay: 2000 },
  };

  const config11 = {
    effect: "fade",
    pagination: { el: ".swiper-pagination", clickable: true },
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
  };

  const config12 = {
    effect: "coverflow",
    coverflowEffect: { rotate: 35, slideShadows: false },
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
  };

  const config13 = {
    parallax: true,
    navigation: {
      prevEl: ".swiper-button-prev",
      nextEl: ".swiper-button-next",
    },
  };

  const config14 = {
    grabCursor: true,
    effect: "creative",
    creativeEffect: {
      prev: {
        shadow: true,
        translate: ["-125%", 0, -800],
        rotate: [0, 0, -90],
      },
      next: { shadow: true, translate: ["125%", 0, -800], rotate: [0, 0, 90] },
    },
  };

  const carousel1 = document.querySelector("#carousel1");
  carousel1._swiper = new Swiper(carousel1, config1);

  const carousel2 = document.querySelector("#carousel2");
  carousel2._swiper = new Swiper(carousel2, config2);

  const carousel3 = document.querySelector("#carousel3");
  carousel3._swiper = new Swiper(carousel3, config3);

  const carousel4 = document.querySelector("#carousel4");
  carousel4._swiper = new Swiper(carousel4, config4);

  const carousel5 = document.querySelector("#carousel5");
  carousel5._swiper = new Swiper(carousel5, config5);

  const carousel6 = document.querySelector("#carousel6");
  carousel6._swiper = new Swiper(carousel6, config6);

  const carousel7 = document.querySelector("#carousel7");
  carousel7._swiper = new Swiper(carousel7, config7);

  const carousel8 = document.querySelector("#carousel8");
  carousel8._swiper = new Swiper(carousel8, config8);

  const carousel9 = document.querySelector("#carousel9");
  carousel9._swiper = new Swiper(carousel9, config9);

  const carousel10 = document.querySelector("#carousel10");
  carousel10._swiper = new Swiper(carousel10, config10);

  const carousel11 = document.querySelector("#carousel11");
  carousel11._swiper = new Swiper(carousel11, config11);

  const carousel12 = document.querySelector("#carousel12");
  carousel12._swiper = new Swiper(carousel12, config12);

  const carousel13 = document.querySelector("#carousel13");
  carousel13._swiper = new Swiper(carousel13, config13);

  const carousel14 = document.querySelector("#carousel14");
  carousel14._swiper = new Swiper(carousel14, config14);
};

window.addEventListener("app:mounted", onLoad, { once: true });
