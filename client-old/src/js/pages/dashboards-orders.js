const onLoad = () => {
  // Overview Chart
  const overviewChartConfig = {
    colors: ["#4C4EE7", "#26E7A6", "#FF9800"],
    series: [
      {
        name: "Orders",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28],
      },
      {
        name: "Completed Orders",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14],
      },
      {
        name: "Returned Orders",
        data: [4, 5, 6, 5, 2, 5, 3, 6, 3],
      },
    ],
    chart: {
      height: 270,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    stroke: {
      show: true,
      width: 2,
      colors: ["transparent"],
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
        bottom: 0,
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

  const overviewChartEl = document.querySelector("#overview-chart");

  setTimeout(() => {
    overviewChartEl._chart = new ApexCharts(
      overviewChartEl,
      overviewChartConfig
    );
    overviewChartEl._chart.render();
  });

  // Budget Chart
  const budgetChartConfig = {
    series: [
      {
        name: "Start",
        data: [44, 55, 41, 25, 22, 56],
      },
      {
        name: "End",
        data: [13, 23, 20, 60, 13, 16],
      },
    ],
    grid: {
      show: false,
      padding: {
        left: 0,
        right: 10,
        bottom: -12,
        top: 0,
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
    chart: {
      type: "bar",
      height: 120,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
      stacked: true,
      stackType: "100%",
    },
    dataLabels: {
      enabled: false,
    },
    fill: {
      colors: ["#0EA5E9", "#e2e8f0"],
    },
    plotOptions: {
      bar: {
        borderRadius: 2,
        horizontal: false,
        columnWidth: 8,
      },
    },
    legend: {
      show: false,
    },
  };

  const budgetChartEl = document.querySelector("#budget-chart");

  setTimeout(() => {
    budgetChartEl._chart = new ApexCharts(budgetChartEl, budgetChartConfig);
    budgetChartEl._chart.render();
  });

  // Income Chart
  const incomeChartConfig = {
    colors: ["#10b981"],
    series: [
      {
        name: "Income",
        data: [20, 50, 30, 60, 33, 75],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 150,
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
        top: -20,
        bottom: -8,
      },
    },
    fill: {
      type: "gradient",
      gradient: {
        shadeIntensity: 1,
        inverseColors: false,
        opacityFrom: 0.45,
        opacityTo: 0.1,
        stops: [20, 100, 100, 100],
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

  const incomeChartEl = document.querySelector("#income-chart");

  setTimeout(() => {
    incomeChartEl._chart = new ApexCharts(incomeChartEl, incomeChartConfig);
    incomeChartEl._chart.render();
  });

  // Expense Chart
  const expenseChartConfig = {
    colors: ["#FF5724"],
    series: [
      {
        name: "Expense",
        data: [82, 25, 60, 30, 50, 20],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 150,
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
        top: -20,
        bottom: -8,
      },
    },
    fill: {
      type: "gradient",
      gradient: {
        shadeIntensity: 1,
        inverseColors: false,
        opacityFrom: 0.45,
        opacityTo: 0.1,
        stops: [20, 100, 100, 100],
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

  const expenseChartEl = document.querySelector("#expense-chart");

  setTimeout(() => {
    expenseChartEl._chart = new ApexCharts(expenseChartEl, expenseChartConfig);
    expenseChartEl._chart.render();
  });

  // Top Seller Chart 1
  const topSeller1Config = {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
        data: [20, 420, 102, 540, 275, 614],
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
        top: -10,
        bottom: 0,
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

  const topSeller1El = document.querySelector("#top-seller-1-chart");

  setTimeout(() => {
    topSeller1El._chart = new ApexCharts(topSeller1El, topSeller1Config);
    topSeller1El._chart.render();
  });

  // Top Seller Chart 2
  const topSeller2Config = {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
        data: [54, 77, 43, 69, 12],
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
        top: -10,
        bottom: 0,
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

  const topSeller2El = document.querySelector("#top-seller-2-chart");

  setTimeout(() => {
    topSeller2El._chart = new ApexCharts(topSeller2El, topSeller2Config);
    topSeller2El._chart.render();
  });

  // Top Seller Chart 3
  const topSeller3Config = {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
        data: [0, 20, 10, 30, 20, 50],
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
        top: -10,
        bottom: 0,
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

  const topSeller3El = document.querySelector("#top-seller-3-chart");

  setTimeout(() => {
    topSeller3El._chart = new ApexCharts(topSeller3El, topSeller3Config);
    topSeller3El._chart.render();
  });

  // Top Seller Chart 4
  const topSeller4Config = {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
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
        top: -10,
        bottom: 0,
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

  const topSeller4El = document.querySelector("#top-seller-4-chart");

  setTimeout(() => {
    topSeller4El._chart = new ApexCharts(topSeller4El, topSeller4Config);
    topSeller4El._chart.render();
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

  // Overview Menu
  new Popper("#overview-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Budget Menu
  new Popper("#budget-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Income Menu
  new Popper("#income-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Expense Menu
  new Popper("#expense-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Top Seller Menu
  new Popper("#top-seller-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Traffic Menu
  new Popper("#traffic-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Orders Table Menu
  new Popper(
    "#orders-table-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Orders Overview Tabs
  new Tab("#overview-tab");
};

window.addEventListener("app:mounted", onLoad, { once: true });
