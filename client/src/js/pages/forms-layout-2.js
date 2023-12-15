const onLoad = () => {
  //  Tom Select (Choose category)
  const configCategory = {
    create: true,
    sortField: { field: "text", direction: "asc" },
  };
  const categoryEl = document.querySelector("#category");
  categoryEl._tom = new Tom(categoryEl, configCategory);

  //   Images (filepond)
  const imagesEl = document.querySelector("#images");
  imagesEl._filepond = FilePond.create(imagesEl);
};

window.addEventListener("app:mounted", onLoad, { once: true });
