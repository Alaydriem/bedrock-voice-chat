const onLoad = () => {
  //   Basic Datetimepicker
  const config1 = { enableTime: true };
  const flatpickr1 = document.querySelector("#flatpickr1");
  flatpickr1._datetimepicker = flatpickr(flatpickr1, config1);

  //   Limit Time
  const config2 = { enableTime: true, minTime: "16:00", maxTime: "22:00" };
  const flatpickr2 = document.querySelector("#flatpickr2");
  flatpickr2._datetimepicker = flatpickr(flatpickr2, config2);

  //   Inline Datetimepicker
  const config3 = { enableTime: true, inline: true };
  const flatpickr3 = document.querySelector("#flatpickr3");
  flatpickr3._datetimepicker = flatpickr(flatpickr3, config3);
};

window.addEventListener("app:mounted", onLoad, { once: true });
