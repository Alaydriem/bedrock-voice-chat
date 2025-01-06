const onLoad = () => {
  new Accordion(document.querySelector("#accordion_exapmple_1"), {
    onlyChildNodes: false,
    duration: 200,
  });

  new Accordion(document.querySelector("#accordion_exapmple_2"), {
    onlyChildNodes: false,
    duration: 200,
  });

  new Accordion(document.querySelector("#accordion_exapmple_3"), {
    onlyChildNodes: false,
    duration: 200,
  });

  new Accordion(document.querySelector("#accordion_exapmple_4"), {
    onlyChildNodes: false,
    duration: 200,
  });

  new Accordion(document.querySelector("#accordion_exapmple_5"), {
    onlyChildNodes: false,
    duration: 200,
    openOnInit: [0],
  });

  new Accordion(document.querySelector("#accordion_exapmple_6"), {
    onlyChildNodes: false,
    duration: 200,
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
