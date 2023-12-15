const onLoad = () => {
  // Sales Overview Tabs
  new Tab("#sales-tab");

  // Sales Month chart
  const salesChartConfig = {
    colors: ["#4467EF"],
    chart: {
      height: 60,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Sales",
        data: [654, 820, 102, 540, 154, 614],
      },
    ],
    dataLabels: {
      enabled: false,
    },
    stroke: {
      curve: "smooth",
      width: 3,
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: -20,
        bottom: -10,
      },
    },
    xaxis: {
      show: false,
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
    yaxis: {
      show: false,
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

  const salesChartEl = document.querySelector("#salesMonthChart");

  setTimeout(() => {
    salesChartEl._chart = new ApexCharts(salesChartEl, salesChartConfig);
    salesChartEl._chart.render();
  });

  // Sales Overview Chart
  const salesOverviewConfig = {
    colors: ["#4C4EE7", "#0EA5E9"],
    series: [
      {
        name: "Sales",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28, 45, 35, 50],
      },
      {
        name: "Profit",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14, 25, 20, 25],
      },
    ],
    chart: {
      height: 255,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    plotOptions: {
      bar: {
        borderRadius: 4,
        barHeight: "90%",
        columnWidth: "35%",
      },
    },
    legend: {
      show: false,
    },
    xaxis: {
      categories: [
        "Jan",
        "Feb",
        "Mar",
        "Apr",
        "May",
        "Jun",
        "Jul",
        "Aug",
        "Sep",
        "Oct",
        "Nov",
        "Dec",
      ],
      labels: {
        hideOverlappingLabels: false,
      },
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
        bottom: -10,
      },
    },
    yaxis: {
      show: false,
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
    responsive: [
      {
        breakpoint: 850,
        options: {
          plotOptions: {
            bar: {
              columnWidth: "55%",
            },
          },
        },
      },
    ],
  };

  const salesOverviewEl = document.querySelector("#salesOverview");

  setTimeout(() => {
    salesOverviewEl._chart = new ApexCharts(
      salesOverviewEl,
      salesOverviewConfig
    );
    salesOverviewEl._chart.render();
  });

  // Bandwith Report chart
  bandwidthConfig = {
    colors: ["#4467EF"],

    series: [
      {
        name: "Traffic",
        data: [
          8107.85, 8128.0, 8122.9, 8165.5, 8340.7, 8423.7, 8423.5, 8514.3,
          8481.85, 8487.7, 8506.9, 8626.2, 8668.95, 8602.3, 8607.55, 8512.9,
          8496.25, 8600.65, 8881.1, 9340.85,
        ],
      },
    ],
    chart: {
      type: "area",
      height: 220,
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
    stroke: {
      curve: "smooth",
      width: 2,
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: -28,
        bottom: -15,
      },
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

  const bandwidthEl = document.querySelector("#bandwidth-chart");

  setTimeout(() => {
    bandwidthEl._chart = new ApexCharts(bandwidthEl, bandwidthConfig);
    bandwidthEl._chart.render();
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

  // Project Status Menu
  new Popper(
    "#project-status-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Satisfaction Menu
  new Popper(
    "#satisfaction-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Bandwidth Report Menu
  new Popper("#bandwidth-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Users Activity Menu
  new Popper(
    "#users-activity-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
