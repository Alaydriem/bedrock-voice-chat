const onLoad = () => {
  new Accordion(document.querySelector("#collapse_exapmple_1"), {
    onlyChildNodes: false,
    duration: 200,
    showMultiple: true,
  });

  new Accordion(document.querySelector("#collapse_exapmple_2"), {
    onlyChildNodes: false,
    duration: 200,
    showMultiple: true,
  });

  new Accordion(document.querySelector("#collapse_exapmple_3"), {
    onlyChildNodes: false,
    duration: 200,
    showMultiple: true,
  });

  new Accordion(document.querySelector("#collapse_exapmple_4"), {
    onlyChildNodes: false,
    duration: 200,
    showMultiple: true,
  });

  new Accordion(document.querySelector("#collapse_exapmple_5"), {
    onlyChildNodes: false,
    duration: 200,
    openOnInit: [0],
    showMultiple: true,
  });

  new Accordion(document.querySelector("#collapse_exapmple_6"), {
    onlyChildNodes: false,
    duration: 200,
    showMultiple: true,
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
