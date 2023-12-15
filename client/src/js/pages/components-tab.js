const onLoad = () => {
  new Tab(document.querySelector("#tab-wrapper-1"));
  new Tab(document.querySelector("#tab-wrapper-2"));
  new Tab(document.querySelector("#tab-wrapper-3"));
  new Tab(document.querySelector("#tab-wrapper-4"));
  new Tab(document.querySelector("#tab-wrapper-5"));
};

window.addEventListener("app:mounted", onLoad, { once: true });
