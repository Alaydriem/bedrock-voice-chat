const onLoad = () => {
  //   Basic Datetimepicker
  const config1 = { enableTime: true, noCalendar: true, dateFormat: "H:i" };
  const flatpickr1 = document.querySelector("#flatpickr1");
  flatpickr1._timepicker = flatpickr(flatpickr1, config1);

  //    24h clock
  const config2 = {
    enableTime: true,
    noCalendar: true,
    dateFormat: "H:i",
    time_24hr: true,
  };
  const flatpickr2 = document.querySelector("#flatpickr2");
  flatpickr2._timepicker = flatpickr(flatpickr2, config2);

  //   Limit Time
  const config3 = {
    enableTime: true,
    noCalendar: true,
    dateFormat: "H:i",
    minTime: "15:00",
    maxTime: "22:30",
  };

  const flatpickr3 = document.querySelector("#flatpickr3");
  flatpickr3._timepicker = flatpickr(flatpickr3, config3);

  //  Default Time
  const config4 = {
    enableTime: true,
    noCalendar: true,
    dateFormat: "H:i",
    defaultDate: "13:13",
  };

  const flatpickr4 = document.querySelector("#flatpickr4");
  flatpickr4._timepicker = flatpickr(flatpickr4, config4);
  
  //   Inline Time
  const config5 = {
    inline: true,
    enableTime: true,
    noCalendar: true,
    dateFormat: "h : m",
    defaultDate: dayjs().format("h : m"),
  };

  const flatpickr5 = document.querySelector("#flatpickr5");
  flatpickr5._timepicker = flatpickr(flatpickr5, config5);
};

window.addEventListener("app:mounted", onLoad, { once: true });
