const onLoad = () => {
  // Post Content (Quill Editor)
  const configPostConent = {
    modules: {
      toolbar: [
        ["bold", "italic", "underline", "strike"], // toggled buttons
        ["blockquote", "code-block"],
        [{ header: 1 }, { header: 2 }], // custom button values
        [{ list: "ordered" }, { list: "bullet" }],
        [{ script: "sub" }, { script: "super" }], // superscript/subscript
        [{ indent: "-1" }, { indent: "+1" }], // outdent/indent
        [{ direction: "rtl" }], // text direction
        [{ size: ["small", false, "large", "huge"] }], // custom dropdown
        [{ header: [1, 2, 3, 4, 5, 6, false] }],
        [{ color: [] }, { background: [] }], // dropdown with defaults from theme
        [{ font: [] }],
        [{ align: [] }],
        ["clean"], // remove formatting button
      ],
    },
    placeholder: "Enter your content...",
    theme: "snow",
  };

  const postConentEl = document.querySelector("#postConent");
  postConentEl._editor = new Quill(postConentEl, configPostConent);

  // Post Images (Filepond)
  const postImagesEl = document.querySelector("#postImages");
  postImagesEl._filepond = FilePond.create(postImagesEl);

  // Post Authors (Tom Select)
  const configPostAuthor = {
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
  const postAuthorEl = document.querySelector("#postAuthor");
  postAuthorEl._tom = new Tom(postAuthorEl, configPostAuthor);

  // Post Categories (Tom Select)
  configPostCategory = {
    create: false,
    sortField: { field: "text", direction: "asc" },
  };

  const postCategoryEl = document.querySelector("#postCategory");
  postCategoryEl._tom = new Tom(postCategoryEl, configPostCategory);

  // Post Tags (Tom Select)
  const postTagsEl = document.querySelector("#postTags");
  postTagsEl._tom = new Tom(postTagsEl, { create: true });

  // Post Publish Date (Flatpickr)
  const postPublishDateEl = document.querySelector("#postPublishDate");
  postPublishDateEl._datepicker = flatpickr(postPublishDateEl);
};

window.addEventListener("app:mounted", onLoad, { once: true });
