const onLoad = () => {
  const toggle = document.querySelector("#monochromeToggle");

  toggle.addEventListener("change", () => $monochromemode.toggle());

  window.addEventListener("change:monochrome", (evt) => {
    toggle.checked = evt.detail.currentMode === "monochrome";
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
