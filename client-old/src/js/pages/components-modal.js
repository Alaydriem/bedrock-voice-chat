const onLoad = () => {
  new Modal("#modal1");
  new Modal("#modal2");
  new Modal("#modal3");
  new Modal("#modal4");
  new Modal("#modal5");
  new Modal("#modal6");
};

window.addEventListener("app:mounted", onLoad, { once: true });
