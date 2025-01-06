const onLoad = () => {
  // Dismissable alert 1
  const alert1 = document.querySelector("#alert-dismissable-1");

  alert1.querySelector("[data-remove-alert]").addEventListener("click", () => {
    alert1.classList.add(...alert1.dataset.dismissClass.trim().split(" "));
    setTimeout(() => alert1.remove(), 300);
  });

  // Dismissable alert 2
  const alert2 = document.querySelector("#alert-dismissable-2");
  const popperConfig = {
    placement: "bottom-end",
  };

  alert2.querySelector("[data-remove-alert]").addEventListener("click", () => {
    alert2.classList.add(...alert2.dataset.dismissClass.trim().split(" "));
    setTimeout(() => alert2.remove(), 300);
  });

  new Popper(
    "#alert-dismissable-2-popper",
    "#alert-dismissable-2-popper-ref",
    "#alert-dismissable-2-popper-box",
    popperConfig
  );

  // Dismissable alert 3
  const alert3 = document.querySelector("#alert-dismissable-3");
  
  alert3.querySelector("[data-remove-alert]").addEventListener("click", () => {
    alert3.classList.add(...alert3.dataset.dismissClass.trim().split(" "));
    setTimeout(() => alert3.remove(), 300);
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
