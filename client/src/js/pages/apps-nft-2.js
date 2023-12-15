const onLoad = () => {
  new Tab("#feed-tab");
};

window.addEventListener("app:mounted", onLoad, { once: true });
