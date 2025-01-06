const onLoad = () => {
  // Sidebar Label Menu
  new Popper("#sidebar-label-menu", ".popper-ref", ".popper-root", {
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

  // Todolist Drag & Drop
  const todoList = document.querySelector("#todo-list");

  todoList._sortable = Sortable.create(todoList, {
    animation: 200,
    easing: "cubic-bezier(0, 0, 0.2, 1)",
    direction: "vertical",
    delay: 150,
    delayOnTouchOnly: true,
  });

  // Todo Checkbox
  const todoCheckbox = document.querySelectorAll(".todo-checkbox");

  todoCheckbox.forEach((node) =>
    node.addEventListener("click", (e) => e.stopPropagation())
  );

  // Edit Todo Drawer
  new Drawer("#edit-todo-drawer");

  // Edit Todo "Tags"
  const editTodoTags = document.querySelector("#edit-todo-tags");
  editTodoTags._tom = new Tom(editTodoTags);

  // Edit Todo "Due Date"
  const editTodoDueDate = document.querySelector("#edit-todo-due-date");
  editTodoDueDate._datepicker = flatpickr(editTodoDueDate, {
    defaultDate: "2020-01-05",
  });

  // Edit Todo "Assigned to"
  const assignedTodoConfig = {
    valueField: "id",
    searchField: "title",
    options: [
      {
        id: 1,
        name: "John Doe",
        job: "Web designer",
        src: "images/200x200.png",
      },
      {
        id: 2,
        name: "Emilie Watson",
        job: "Developer",
        src: "images/200x200.png",
      },
      {
        id: 3,
        name: "Nancy Clarke",
        job: "Software Engineer",
        src: "images/200x200.png",
      },
    ],
    placeholder: "Select the user",
    render: {
      option: function (data, escape) {
        return `<div class="flex space-x-3">
                      <div class="avatar w-8 h-8">
                        <img class="rounded-full" src="${escape(
                          data.src
                        )}" alt="avatar"/>
                      </div>
                      <div class="flex flex-col">
                        <span> ${escape(data.name)}</span>
                        <span class="text-xs opacity-80"> ${escape(
                          data.job
                        )}</span>
                      </div>
                    </div>`;
      },
      item: function (data, escape) {
        return `<span class="inline-flex items-center">
        <span class="avatar w-6 h-6">
            <img class="rounded-full" src="${escape(data.src)}" alt="avatar">
        </span>
        <span class="mx-2">${escape(data.name)}</span>
      </span>`;
      },
    },
  };

  const editTodoAssigned = document.querySelector("#edit-todo-assigned");
  editTodoAssigned._tom = new Tom(editTodoAssigned, assignedTodoConfig);

  // Edit Todo Description
  const descTodoConfig = {
    modules: {
      toolbar: [
        ["bold", "italic", "underline"],
        [
          { list: "ordered" },
          { list: "bullet" },
          { header: 1 },
          { background: [] },
        ],
      ],
    },
    placeholder: "Enter your content...",
    theme: "snow",
  };

  const editTodoDesc = document.querySelector("#edit-todo-description");
  editTodoDesc._quill = new Quill(editTodoDesc, descTodoConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
