export function toggleCode(e) {
  const card = e.target.closest(".card");
  const codeWrapper = card.querySelector(".code-wrapper");

  e.target.checked
    ? codeWrapper.classList.remove("hidden")
    : codeWrapper.classList.add("hidden");
}

export function getBrwoserScrollbarWidth() {
  return window.innerWidth - document.documentElement.clientWidth;
}

export function getCurrentLocation() {
  return [location.protocol, "//", location.host, location.pathname].join("");
}

export function leaveAnimation(node, cb, leaveClass = "animate:leave") {
  const styles = window.getComputedStyle(node);
  if (styles.getPropertyValue("display") === "none") return;

  node.classList.add(leaveClass);

  node.addEventListener(
    "animationend",
    () => {
      cb();
      node.classList.remove(leaveClass);
    },
    { once: true }
  );
}
