const onLoad = () => {
  const dropdownConfig = {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  };

  new Popper(
    "#dropdown-wrapper1",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-wrapper2",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-table-1",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-table-2",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-table-3",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-table-4",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Table Collapse
  window.tableCollapse = new Accordion(
    document.querySelector("#table-collapse"),
    {
      onlyChildNodes: false,
      duration: 200,
      showMultiple: true,
    }
  );

  // Table Filter collapse
  new Accordion(document.querySelector("#table-filter"), {
    onlyChildNodes: false,
    duration: 200,
  });

  // Filter From datepicker
  const filterFormDatepicker = document.querySelector(
    "#filter-from-datepicker"
  );

  // Filter To datepicker
  filterFormDatepicker._datepicker = flatpickr(filterFormDatepicker);

  const filterToDatepicker = document.querySelector("#filter-to-datepicker");
  filterToDatepicker._datepicker = flatpickr(filterToDatepicker);
};

window.addEventListener("app:mounted", onLoad, { once: true });
