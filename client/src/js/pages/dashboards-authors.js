const onLoad = () => {
  // Visitors Chart
  const visitorsConfig = {
    colors: ["#10b981"],
    series: [
      {
        name: "Visitors",
        data: [35, 20, 45, 30, 55, 27, 45],
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
        opacityFrom: 0.35,
        opacityTo: 0.05,
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

  const visitorsEl = document.querySelector("#visitors-chart");

  setTimeout(() => {
    visitorsEl._chart = new ApexCharts(visitorsEl, visitorsConfig);
    visitorsEl._chart.render();
  });

  // Members Chart
  const membersConfig = {
    colors: ["#ff5724"],
    series: [
      {
        name: "Members",
        data: [65, 40, 60, 35, 56, 42],
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
        opacityFrom: 0.35,
        opacityTo: 0.05,
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

  const membersEl = document.querySelector("#members-chart");

  setTimeout(() => {
    membersEl._chart = new ApexCharts(membersEl, membersConfig);
    membersEl._chart.render();
  });

  // Author Chart 1
  const authors1Config = {
    series: [
      {
        name: "Posts",
        data: [1765, 2357, 4215, 3971, 3841, 4221],
      },
    ],
    colors: ["#4467EF"],
    chart: {
      height: 85,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    grid: {
      padding: {
        left: -18,
        right: 0,
        top: -30,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 8,
        columnWidth: "55%",
      },
    },
    dataLabels: {
      enabled: false,
    },
    xaxis: {
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

  const authors1El = document.querySelector("#author-chart-1");

  setTimeout(() => {
    authors1El._chart = new ApexCharts(authors1El, authors1Config);
    authors1El._chart.render();
  });

  // Author Chart 2
  const authors2Config = {
    series: [
      {
        name: "Posts",
        data: [2357, 4215, 1765, 4221, 3841, 5665],
      },
    ],
    colors: ["#f000b9"],
    chart: {
      height: 85,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    grid: {
      padding: {
        left: -18,
        right: 0,
        top: -30,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 8,
        columnWidth: "55%",
      },
    },
    dataLabels: {
      enabled: false,
    },

    xaxis: {
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

  const authors2El = document.querySelector("#author-chart-2");

  setTimeout(() => {
    authors2El._chart = new ApexCharts(authors2El, authors2Config);
    authors2El._chart.render();
  });

  // Author Chart 3
  const authors3Config = {
    series: [
      {
        name: "Posts",
        data: [6153, 7020, 5659, 3422, 5439, 6081],
      },
    ],
    colors: ["#10b981"],
    chart: {
      height: 85,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    grid: {
      padding: {
        left: -18,
        right: 0,
        top: -30,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 8,
        columnWidth: "55%",
      },
    },
    dataLabels: {
      enabled: false,
    },

    xaxis: {
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

  const authors3El = document.querySelector("#author-chart-3");

  setTimeout(() => {
    authors3El._chart = new ApexCharts(authors3El, authors3Config);
    authors3El._chart.render();
  });

  // Author Chart 4
  const authors4Config = {
    series: [
      {
        name: "Posts",
        data: [1499, 2303, 2857, 1791, 2194, 1351],
      },
    ],
    colors: ["#ff5724"],
    chart: {
      height: 85,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    grid: {
      padding: {
        left: -18,
        right: 0,
        top: -30,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 8,
        columnWidth: "55%",
      },
    },
    dataLabels: {
      enabled: false,
    },

    xaxis: {
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

  const authors4El = document.querySelector("#author-chart-4");

  setTimeout(() => {
    authors4El._chart = new ApexCharts(authors4El, authors4Config);
    authors4El._chart.render();
  });

  // Author Chart 5
  const authors5Config = {
    series: [
      {
        name: "Posts",
        data: [1765, 2357, 4215, 3971, 3841, 4221],
      },
    ],
    colors: ["#4467EF"],
    chart: {
      height: 85,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    grid: {
      padding: {
        left: -18,
        right: 0,
        top: -30,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 8,
        columnWidth: "55%",
      },
    },
    dataLabels: {
      enabled: false,
    },

    xaxis: {
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

  const authors5El = document.querySelector("#author-chart-5");

  setTimeout(() => {
    authors5El._chart = new ApexCharts(authors5El, authors5Config);
    authors5El._chart.render();
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
