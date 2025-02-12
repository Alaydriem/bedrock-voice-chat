export const tomSelect = {
  author: {
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
  },

  assignedTodo: {
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
            <img class="rounded-full" src="${escape(
              data.src
            )}" alt="avatar">
        </span>
        <span class="mx-2">${escape(data.name)}</span>
      </span>`;
      },
    },
  },
};
