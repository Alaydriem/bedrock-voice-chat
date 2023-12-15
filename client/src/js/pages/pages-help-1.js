const onLoad = () => {
  const faq = document.querySelector("#faq");

  faq._accordion = new Accordion(faq, {
    onlyChildNodes: false,
    duration: 200,
    openOnInit: [0],
  });
};
window.addEventListener("app:mounted", onLoad, { once: true });
