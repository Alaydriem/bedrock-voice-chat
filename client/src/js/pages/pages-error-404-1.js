const onLoad = () => {
  const mainConentEl = document.querySelector("main");
  const darkImage = document.querySelector("#hero-image-dark");
  const lightImage = document.querySelector("#hero-image-light");

  if ($darkmode.currentMode === "dark") {
    mainConentEl.style.backgroundImage = `url('./images/illustrations/ufo-bg-dark.svg')`;
    lightImage.classList.add("hidden");
  } else {
    mainConentEl.style.backgroundImage = `url('./images/illustrations/ufo-bg.svg')`;
    darkImage.classList.add("hidden");
  }
};

window.addEventListener("app:mounted", onLoad, { once: true });
