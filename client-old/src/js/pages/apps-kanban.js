const onLoad = () => {
  // Sidebar "Workspace" collapse
  new Accordion("#sidebar-workspace", {
    duration: 200,
    openOnInit: [0],
  });

  // Setting Drawer
  new Drawer("#kanban-setting-drawer");

  // Drawer Actions Collapse
  new Accordion("#drawer-actions-collapse", {
    duration: 200,
    openOnInit: [0],
  });

  // Drawer Activities Collapse
  new Accordion("#drawer-activities-collapse", {
    duration: 200,
    openOnInit: [0],
  });

  // Task Group Drag & Drop
  const tasksGroup = document.querySelector("#tasks-group");

  tasksGroup._sortable = Sortable.create(tasksGroup, {
    animation: 200,
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    delay: 150,
    delayOnTouchOnly: true,
    draggable: ".board-draggable",
    handle: ".board-draggable-handler",
  });

  // Task Progress Drag & Drop
  const tasksProgressList = document.querySelector("#tasks-progress-list");

  tasksProgressList._sortable = Sortable.create(tasksProgressList, {
    animation: 200,
    group: "board-cards",
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    direction: "vertical",
    delay: 150,
    delayOnTouchOnly: true,
  });

  // Task Pending Drag & Drop
  const tasksPendingList = document.querySelector("#tasks-pending-list");

  tasksPendingList._sortable = Sortable.create(tasksPendingList, {
    animation: 200,
    group: "board-cards",
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    direction: "vertical",
    delay: 150,
    delayOnTouchOnly: true,
  });

  // Task Pending Drag & Drop
  const tasksReviewList = document.querySelector("#tasks-review-list");

  tasksReviewList._sortable = Sortable.create(tasksReviewList, {
    animation: 200,
    group: "board-cards",
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    direction: "vertical",
    delay: 150,
    delayOnTouchOnly: true,
  });

  // Task Pending Drag & Drop
  const tasksSuccessList = document.querySelector("#tasks-success-list");

  tasksSuccessList._sortable = Sortable.create(tasksSuccessList, {
    animation: 200,
    group: "board-cards",
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    direction: "vertical",
    delay: 150,
    delayOnTouchOnly: true,
  });

  const taskMenuConfig = {
    placement: "bottom-end",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  };

  // Task Progress Menu
  new Popper(
    "#tasks-progress-menu",
    ".popper-ref",
    ".popper-root",
    taskMenuConfig
  );

  // Task Pending Menu
  new Popper(
    "#tasks-pending-menu",
    ".popper-ref",
    ".popper-root",
    taskMenuConfig
  );

  // Task Review Menu
  new Popper(
    "#tasks-review-menu",
    ".popper-ref",
    ".popper-root",
    taskMenuConfig
  );

  // Task Success Menu
  new Popper(
    "#tasks-success-menu",
    ".popper-ref",
    ".popper-root",
    taskMenuConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
