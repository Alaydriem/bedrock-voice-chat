const onLoad = () => {
  const onSuccess = () =>
    $notification({ text: "Text Copied", variant: "success" });

  const onError = () => $notification({ text: "Error", variant: "error" });

  const contentEl1 = document.querySelector("#clipboardContent1");
  const contentEl2 = document.querySelector("#clipboardContent2");

  const copyBtn1 = document.querySelector("#getContent1");
  const copyBtn2 = document.querySelector("#getContent2");

  copyBtn1.addEventListener("click", () => {
    $clipboard({
      content: contentEl1.innerText,
      success: onSuccess,
      error: onError,
    });
  });

  copyBtn2.addEventListener("click", () => {
    $clipboard({
      content: contentEl2.value,
      success: onSuccess,
      error: onError,
    });
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
