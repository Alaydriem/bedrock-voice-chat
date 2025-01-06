const onLoad = () => {
  // Basic Text Editor
  const config1 = {
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

  const editor1 = document.querySelector("#editor1");
  editor1._editor = new Quill(editor1, config1);

  //   Filled Header
  const config2 = {
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

  const editor2 = document.querySelector("#editor2");
  editor2._editor = new Quill(editor2, config2);

  //   Filled Header
  const config3 = {
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

  const editor3 = document.querySelector("#editor3");
  editor3._editor = new Quill(editor3, config3);

  //   Customizing Config
  const config4 = {
    modules: {
      toolbar: [
        ["bold", "italic", "underline", "strike"],
        ["blockquote", "code-block"],
        [{ header: [1, 2, 3, 4, 5, 6, false] }],
        [{ color: [] }, { background: [] }],
        ["clean"],
      ],
    },
    placeholder: "Enter your content...",
    theme: "snow",
  };

  const editor4 = document.querySelector("#editor4");
  editor4._editor = new Quill(editor4, config4);
};

window.addEventListener("app:mounted", onLoad, { once: true });
