const onLoad = () => {
  // Working Hours Chart
  const workingHrConfig = {
    colors: ["#0EA5E9"],
    series: [70],
    chart: {
      height: 210,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          margin: 0,
          size: "70%",
        },
        dataLabels: {
          name: {
            show: false,
          },
          value: {
            show: true,
            color: "#333",
            offsetY: 10,
            fontSize: "24px",
            fontWeight: 600,
          },
        },
      },
    },
    grid: {
      show: false,
      padding: {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
  };
  const workingHrEl = document.querySelector("#working-hr-chart");

  setTimeout(() => {
    workingHrEl._chart = new ApexCharts(workingHrEl, workingHrConfig);
    workingHrEl._chart.render();
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

  // Working Hours Menu
  new Popper("#working-hr-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Mwdia Menu
  new Popper("#media-menu", ".popper-ref", ".popper-root", dropdownConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
