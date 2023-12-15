const onLoad = () => {
  // Watchlist 1 Chart
  const watchlist1Config = {
    colors: ["#F7931A"],
    chart: {
      height: 60,
      width: 120,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Stat",
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
        top: -28,
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

  const watchlist1El = document.querySelector("#watchlist-1-chart");

  setTimeout(() => {
    watchlist1El._chart = new ApexCharts(watchlist1El, watchlist1Config);
    watchlist1El._chart.render();
  });

  // Watchlist 2 Chart
  const watchlist2Config = {
    colors: ["#627EEA"],
    chart: {
      height: 60,
      width: 120,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Stat",
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
        top: -28,
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

  const watchlist2El = document.querySelector("#watchlist-2-chart");

  setTimeout(() => {
    watchlist2El._chart = new ApexCharts(watchlist2El, watchlist2Config);
    watchlist2El._chart.render();
  });

  // Watchlist 3 Chart
  const watchlist3Config = {
    colors: ["#3AC5BC"],
    chart: {
      height: 60,
      width: 120,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Stat",
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
        top: -28,
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

  const watchlist3El = document.querySelector("#watchlist-3-chart");

  setTimeout(() => {
    watchlist3El._chart = new ApexCharts(watchlist3El, watchlist3Config);
    watchlist3El._chart.render();
  });

  // Watchlist 4 Chart
  const watchlist4Config = {
    colors: ["#4073C3"],
    chart: {
      height: 60,
      width: 120,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Stat",
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
        top: -28,
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

  const watchlist4El = document.querySelector("#watchlist-4-chart");

  setTimeout(() => {
    watchlist4El._chart = new ApexCharts(watchlist4El, watchlist4Config);
    watchlist4El._chart.render();
  });

  // Transactions Chart
  const transactionsConfig = {
    colors: ["#0EA5E9", "#F000B9"],
    series: [
      {
        name: "Income",
        data: [28, 45, 35, 50, 32, 48, 31],
      },
      {
        name: "outcome",
        data: [14, 25, 20, 25, 12, 16, 12],
      },
    ],
    chart: {
      height: 228,
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
      categories: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul"],

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

  const transactionsEl = document.querySelector("#transactions-chart");

  setTimeout(() => {
    transactionsEl._chart = new ApexCharts(transactionsEl, transactionsConfig);
    transactionsEl._chart.render();
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

  // Balance Menu
  new Popper("#balance-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Accounts Menu
  new Popper("#accounts-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Watchlist Menu
  new Popper("#watchlist-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Exchange Menu
  new Popper("#exchange-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Activities Menu
  new Popper("#activities-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Transactions Menu
  new Popper(
    "#transactions-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Exchange Tab
  new Tab("#exchange-tab");
};

window.addEventListener("app:mounted", onLoad, { once: true });
