const onLoad = () => {
  // Daily Visitors Chart
  const visitorsConfig = {
    colors: ["#FF5724"],
    series: [
      {
        data: [0, 20, 10, 30, 20, 50],
      },
    ],
    chart: {
      type: "line",
      stacked: false,
      height: 150,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },

      zoom: {
        enabled: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    grid: {
      show: false,
      padding: {
        left: 0,
        right: 0,
      },
    },
    stroke: {
      width: 3,
      curve: "smooth",
    },
    tooltip: {
      shared: true,
    },
    legend: {
      show: false,
    },
    yaxis: {
      show: false,
    },
    xaxis: {
      labels: {
        show: false,
      },
      axisTicks: {
        show: false,
      },
      axisBorder: {
        show: false,
      },
    },
  };

  const visitorsEl = document.querySelector("#visitors-chart");

  setTimeout(() => {
    visitorsEl._chart = new ApexCharts(visitorsEl, visitorsConfig);
    visitorsEl._chart.render();
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

  // Labels Menu
  new Popper("#labels-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Statistic Menu
  new Popper("#statistic-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Maybe you know Menu
  new Popper("#you-know-menu", ".popper-ref", ".popper-root", dropdownConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
