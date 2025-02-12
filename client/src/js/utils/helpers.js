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
