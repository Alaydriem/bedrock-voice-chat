const onLoad = () => {
    const faqGroup1 = document.querySelector("#faqGroup1");
    const faqGroup2 = document.querySelector("#faqGroup2");
    const faqGroup3 = document.querySelector("#faqGroup3");
  
    faqGroup1._accordion = new Accordion(faqGroup1, {
      onlyChildNodes: false,
      duration: 200,
      openOnInit: [0],
    });
  
    faqGroup2._accordion = new Accordion(faqGroup2, {
      onlyChildNodes: false,
      duration: 200,
      openOnInit: [0],
    });
  
    faqGroup3._accordion = new Accordion(faqGroup3, {
      onlyChildNodes: false,
      duration: 200,
      openOnInit: [0],
    });
    };
  window.addEventListener("app:mounted", onLoad, { once: true });
  