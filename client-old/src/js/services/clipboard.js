export default (userOptions) => {
  const options = {
    content: userOptions.content,
    onSuccess: userOptions.success ?? function () {},
    onError: userOptions.error ?? function () {},
  };

  if (userOptions.content === "") return;

  if (typeof userOptions.content === "function") {
    userOptions.content = userOptions.content();
  }

  navigator.clipboard.writeText(options.content).then(
    function () {
      options.onSuccess();
    },
    function (err) {
      options.onError(err);
    }
  );
};
