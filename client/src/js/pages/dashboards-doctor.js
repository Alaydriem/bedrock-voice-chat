const onLoad = () => {
  // Number Of Parients Chart
  const parientsConfig = {
    colors: ["#0EA5E9", "#F000B9"],
    series: [
      {
        name: "Man",
        data: [28, 45, 35, 50, 32],
      },
      {
        name: "Woman",
        data: [14, 25, 20, 25, 12],
      },
    ],
    chart: {
      height: 210,
      type: "bar",
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    plotOptions: {
      bar: {
        borderRadius: 5,
        barHeight: "90%",
        columnWidth: "40%",
      },
    },
    legend: {
      show: false,
    },
    xaxis: {
      categories: ["Jan", "Feb", "Mar", "Apr", "May"],

      axisBorder: {
        show: false,
      },
      axisTicks: {
        show: false,
      },
      tooltip: {
        enabled: false,
      },
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: 0,
        bottom: -8,
      },
    },
    yaxis: {
      axisBorder: {
        show: false,
      },
      axisTicks: {
        show: false,
      },
      labels: {
        show: false,
      },
    },
  };

  const parientsEl = document.querySelector("#parients-chart");

  setTimeout(() => {
    parientsEl._chart = new ApexCharts(parientsEl, parientsConfig);
    parientsEl._chart.render();
  });

  // Dropdown Menu Config
  const dropdownConfig = {
    placement: "bottom-end",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  };

  // Next Patient Menu
  new Popper(
    "#next-patient-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  //  Number of Patient Menu
  new Popper("#parients-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Appointments Menu
  new Popper(
    "#appointments-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
