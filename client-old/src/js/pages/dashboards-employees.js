const onLoad = () => {
  // Client Growth Chart
  const clientGrowthConfig = {
    colors: ["#10B981"],
    series: [
      {
        name: "Clients Growth",
        data: [45, 20, 55, 28, 45, 25, 65],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 120,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: -28,
        bottom: -15,
      },
    },
    stroke: {
      width: 2,
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

  const clientGrowthEl = document.querySelector("#client-growth-chart");

  setTimeout(() => {
    clientGrowthEl._chart = new ApexCharts(clientGrowthEl, clientGrowthConfig);
    clientGrowthEl._chart.render();
  });

  // Sales Growth Chart
  const salesGrowthConfig = {
    colors: ["#FF9800"],
    series: [
      {
        name: "Sales Growth",
        data: [35, 20, 45, 30, 55, 27, 45],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 120,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: -28,
        bottom: -15,
      },
    },
    stroke: {
      width: 2,
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

  const salesGrowthEl = document.querySelector("#sales-growth-chart");

  setTimeout(() => {
    salesGrowthEl._chart = new ApexCharts(salesGrowthEl, salesGrowthConfig);
    salesGrowthEl._chart.render();
  });

  // Completed Tasks Chart
  const tasksConfig = {
    colors: ["#0EA5E9"],
    series: [65],
    chart: {
      height: 120,
      width: 120,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          size: "60%",
        },
        dataLabels: {
          name: {
            show: false,
            color: "#fff",
          },
          value: {
            show: true,
            fontSize: "18px",
            offsetY: 4,
          },
        },
      },
    },
    grid: {
      show: false,
      padding: {
        left: -20,
        right: -20,
        top: -20,
        bottom: -23,
      },
    },
    stroke: {
      lineCap: "round",
    },
  };

  const tasksEl = document.querySelector("#tasks-chart");

  setTimeout(() => {
    tasksEl._chart = new ApexCharts(tasksEl, tasksConfig);
    tasksEl._chart.render();
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

  // Client Growth Menu
  new Popper(
    "#client-growth-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Sales Growth Menu
  new Popper(
    "#sales-growth-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Employees Menu
  new Popper("#employees-menu", ".popper-ref", ".popper-root", dropdownConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
