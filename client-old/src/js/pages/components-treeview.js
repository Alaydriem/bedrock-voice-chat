const onLoad = () => {
  new Accordion("#tree1", {
    duration: 200,
    showMultiple: true,
    openOnInit: [1],
  });

  new Accordion("#tree1-1", {
    duration: 200,
    showMultiple: true,
    openOnInit: [0],
  });

  new Accordion("#tree1-1-1", {
    duration: 200,
    showMultiple: true,
    openOnInit: [0],
  });

  new Accordion("#tree1-1-1-1", {
    duration: 200,
    showMultiple: true,
    openOnInit: [0],
  });

  new Accordion("#tree2", {
    duration: 200,
    showMultiple: true,
  });

  new Accordion("#tree2-1", {
    duration: 200,
    showMultiple: true,
  });

  new Accordion("#tree2-2", {
    duration: 200,
    showMultiple: true,
  });

  new Accordion("#tree2-3", {
    duration: 200,
    showMultiple: true,
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
