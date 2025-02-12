import Toastify from "toastify-js";

export default (userOptions) => {
  let [position = "right", gravity = "bottom"] = userOptions.position
    ? userOptions.position.split("-")
    : "";

  let removeTrigger = null;

  const options = {
    duration: userOptions.duration || 5000,
    gravity,
    position,
    text: userOptions.text || "This is a message",
    newWindow: true,
    close: userOptions.hasCloseBtn || false,
    backgroundColor: "",
    className: userOptions.class || "",
    stopOnFocus: userOptions.pauseOnHover || true,
  };

  if (userOptions.link) {
    options.destination = userOptions.link;
  }

  if (userOptions.variant) {
    options.className = `${options.className} ${userOptions.variant}`;
  }

  if (userOptions.content) {
    options.node = document.querySelector(userOptions.content).content.firstElementChild.cloneNode(true);
    options.className += " html";
    options.close = false;
  }

  const toastify = Toastify(options);
  toastify.showToast();

  if (userOptions.content) {
    removeTrigger = toastify.toastElement.querySelector(
      "[data-notification-remove]"
    );
    if (removeTrigger) {
      removeTrigger.addEventListener("click", () =>
        toastify.removeElement(toastify.toastElement)
      );
    }
  }
};
