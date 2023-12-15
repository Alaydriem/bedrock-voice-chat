const onLoad = () => {
  const mainEl = document.querySelector("main.chat-app");
  const historySlide = document.querySelector("#history-slide");
  const chatDetailToggleEl = document.querySelector("#chat-detail-toggle");

  historySlide._swiper = new Swiper(historySlide, {
    slidesPerView: "auto",
    spaceBetween: 10,
    slidesPerGroup: 3,
  });

  const onToggleChatDetail = (isActive) => {
    if (isActive) {
      mainEl.classList.add("lg:mr-80");
      chatDetailToggleEl.classList.add(
        "text-primary",
        "dark:text-accent-light"
      );
    } else {
      mainEl.classList.remove("lg:mr-80");
      chatDetailToggleEl.classList.remove(
        "text-primary",
        "dark:text-accent-light"
      );
    }
  };

  const chatDetailsDrawer = new Drawer("#chat-detail", onToggleChatDetail);

  if ($breakpoint.lgAndUp) {
    chatDetailsDrawer.open();
  }

  window.addEventListener("change:breakpoint", () => {
    if (chatDetailsDrawer.isActive) chatDetailsDrawer.close();
  });

  new Popper("#chat-menu", ".popper-ref", ".popper-root", {
    placement: "bottom-end",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  new Tab("#tab-media");
};

window.addEventListener("app:mounted", onLoad, { once: true });
