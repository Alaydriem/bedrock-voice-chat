const onLoad = () => {
  // Input Tag
  const config1 = {
    create: true,
    plugins: ["caret_position", "input_autogrow"],
  };

  const tom1 = document.querySelector("#tom1");
  tom1._tom = new Tom(tom1, config1);

  // Remove Button
  const config2 = {
    plugins: ["remove_button"],
    create: true,
    onItemRemove: function (val) {
      $notification({ text: `${val} removed` });
    },
  };

  const tom2 = document.querySelector("#tom2");
  tom2._tom = new Tom(tom2, config2);

  // Restore on Backspace
  const config3 = { plugins: ["restore_on_backspace"], create: true };

  const tom3 = document.querySelector("#tom3");
  tom3._tom = new Tom(tom3, config3);

  // Clear Button
  const config4 = {
    plugins: {
      clear_button: {
        title: "Remove all selected options",
      },
    },
    persist: false,
    create: true,
  };

  const tom4 = document.querySelector("#tom4");
  tom4._tom = new Tom(tom4, config4);

  // Single Select
  const config5 = {
    create: true,
    sortField: { field: "text", direction: "asc" },
  };

  const tom5 = document.querySelector("#tom5");
  tom5._tom = new Tom(tom5, config5);

  // Select Multiple
  const tom6 = document.querySelector("#tom6");
  tom6._tom = new Tom(tom6);

  // Custom HTML
  const config7 = {
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
    placeholder: "Select the author",
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
        return `<span class="badge rounded-full bg-primary dark:bg-accent text-white p-px mr-2">
                      <span class="avatar w-6 h-6">
                        <img class="rounded-full" src="${escape(
                          data.src
                        )}" alt="avatar">
                      </span>
                      <span class="mx-2">${escape(data.name)}</span>
                    </span>`;
      },
    },
  };

  const tom7 = document.querySelector("#tom7");
  tom7._tom = new Tom(tom7, config7);

  // Disable Persist
  const config8 = { create: true, persist: false };

  const tom8 = document.querySelector("#tom8");
  tom8._tom = new Tom(tom8, config8);
};

window.addEventListener("app:mounted", onLoad, { once: true });
