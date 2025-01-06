const onLoad = () => {
  // Filemanager Treeview collapse
  new Accordion("#my-files", {
    duration: 200,
    openOnInit: [0],
  });

  // Filemanager Treeview 
  new Accordion("#tree1", {
    duration: 200,
    showMultiple: true,
  });

  new Accordion("#tree1-1", {
    duration: 200,
    showMultiple: true,
  });

  new Accordion("#tree1-2", {
    duration: 200,
    showMultiple: true,
  });

  new Accordion("#tree1-3", {
    duration: 200,
    showMultiple: true,
  });

  // Top header menu
  new Popper("#top-header-menu", ".popper-ref", ".popper-root", {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  // Tab Folders
  new Tab("#tab-folders");

  new Swiper("#tab-folder-recent", { slidesPerView: "auto", spaceBetween: 20 });
  new Swiper("#tab-folder-pinned", { slidesPerView: "auto", spaceBetween: 20 });

  // Folders Table Dropdown
  new Popper("#dropdown-folders-table", ".popper-ref", ".popper-root", {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  // Media Tags Dropdown
  new Popper("#dropdown-tags", ".popper-ref", ".popper-root", {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  // Activity Drawer
  new Drawer("#filemanager-activity-drawer");

  // Drawer Tab
  new Tab("#drawer-tab");
};

window.addEventListener("app:mounted", onLoad, { once: true });
