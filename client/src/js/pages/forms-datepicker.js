const onLoad = () => {
  //   Basic Datepricker
  const flatpickr1 = document.querySelector("#flatpickr1");
  flatpickr1._datepicker = flatpickr(flatpickr1);

  //   Custom Format
  const config2 = { altInput: true, altFormat: "F j, Y", dateFormat: "Y-m-d" };
  const flatpickr2 = document.querySelector("#flatpickr2");
  flatpickr2._datepicker = flatpickr(flatpickr2, config2);

  //    Disabled Date
  const config3 = {
    disable: [
      function (date) {
        return date.getDay() === 0 || date.getDay() === 6;
      },
    ],
    locale: {
      firstDayOfWeek: 1,
    },
  };
  const flatpickr3 = document.querySelector("#flatpickr3");
  flatpickr3._datepicker = flatpickr(flatpickr3, config3);

  //   Multiple Date
  const config4 = {
    mode: "multiple",
    dateFormat: "Y-m-d",
    defaultDate: ["2022-10-10", "2022-10-12", "2022-10-18"],
  };
  const flatpickr4 = document.querySelector("#flatpickr4");
  flatpickr4._datepicker = flatpickr(flatpickr4, config4);

  //   Date Range
  const config5 = {
    mode: "range",
    dateFormat: "Y-m-d",
    defaultDate: ["2016-10-10", "2016-10-20"],
  };
  const flatpickr5 = document.querySelector("#flatpickr5");
  flatpickr5._datepicker = flatpickr(flatpickr5, config5);

  //   External Elements
  const config6 = { wrap: true };
  const flatpickr6 = document.querySelector("#flatpickr6");
  flatpickr6._datepicker = flatpickr(flatpickr6, config6);

  //   Inline Datepicker
  const config7 = { inline: true };
  const flatpickr7 = document.querySelector("#flatpickr7");
  flatpickr7._datepicker = flatpickr(flatpickr7, config7);
};

window.addEventListener("app:mounted", onLoad, { once: true });
