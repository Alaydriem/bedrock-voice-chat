const onLoad = () => {
  // Pages List (Tom Select)
  const pagesList = document.querySelector("#pages-list");
  pagesList._tom = new Tom(pagesList, {
    sortField: { field: "text", direction: "asc" },
  });

  // History Transactions Chart
  const pagesViewConfig = {
    colors: ["#FF9800", "#4C4EE7"],
    series: [
      {
        name: "Previous Period",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14, 25, 20, 25],
      },
      {
        name: "Current Period",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28, 45, 35, 50],
      },
    ],
    chart: {
      height: 280,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    stroke: {
      show: true,
      width: 3,
      colors: ["transparent"],
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
        breakpoint: 1024,
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

  const pagesViewEl = document.querySelector("#pages-view-chart");

  setTimeout(() => {
    pagesViewEl._chart = new ApexCharts(pagesViewEl, pagesViewConfig);
    pagesViewEl._chart.render();
  });

  // Site Overview Chart
  const overviewConfig = {
    colors: ["#0EA5E9"],
    series: [
      {
        name: "High",
        data: [28, 45, 35, 50, 32, 55, 23, 60],
      },
    ],
    chart: {
      parentHeightOffset: 0,
      height: 249,
      type: "area",
      toolbar: {
        show: false,
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
    dataLabels: {
      enabled: false,
    },
    stroke: {
      width: 2,
      curve: "smooth",
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
      categories: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],
    },
    yaxis: {
      labels: {
        offsetX: -12,
        offsetY: 0,
      },
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: -10,
        bottom: 8,
      },
    },
  };

  const overviewEl = document.querySelector("#overview-chart");

  setTimeout(() => {
    overviewEl._chart = new ApexCharts(overviewEl, overviewConfig);
    overviewEl._chart.render();
  });

  // Top Writer Chart 1
  const writer1Config = {
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

  const writer1El = document.querySelector("#writer-chart-1");

  setTimeout(() => {
    writer1El._chart = new ApexCharts(writer1El, writer1Config);
    writer1El._chart.render();
  });

  // Top Writer Chart 2
  const writer2Config = {
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

  const writer2El = document.querySelector("#writer-chart-2");

  setTimeout(() => {
    writer2El._chart = new ApexCharts(writer2El, writer2Config);
    writer2El._chart.render();
  });

  // Top Writer Chart 3
  const writer3Config = {
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

  const writer3El = document.querySelector("#writer-chart-3");

  setTimeout(() => {
    writer3El._chart = new ApexCharts(writer3El, writer3Config);
    writer3El._chart.render();
  });

  // Authors Carousel
  const authorsCarousel = document.querySelector("#authors-carousel");
  authorsCarousel._swiper = new Swiper(authorsCarousel, {
    pagination: { el: ".swiper-pagination", clickable: true },
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

  // Post Rankings Menu
  new Popper(
    "#post-rankings-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Site Overview Menu
  new Popper(
    "#site-overview-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Top Writers Menu
  new Popper(
    "#top-writers-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
