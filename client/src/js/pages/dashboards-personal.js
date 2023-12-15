const onLoad = () => {
  // Earning Chart
  const earningConfig = {
    colors: ["#fff"],
    series: [
      {
        name: "Earning",
        data: [0, 20, 50, 10],
      },
    ],
    chart: {
      type: "line",
      stacked: false,
      height: 60,
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

  const earningEl = document.querySelector("#earning-chart");

  setTimeout(() => {
    earningEl._chart = new ApexCharts(earningEl, earningConfig);
    earningEl._chart.render();
  });

  // Income Chart
  const incomeConfig = {
    series: [
      {
        name: "Income",
        data: [1765, 2357, 4215, 3971, 3841, 4221, 2374, 4212],
      },
    ],
    colors: ["#4467EF"],
    chart: {
      height: 250,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 10,
        columnWidth: "55%",
        dataLabels: {
          position: "top",
        },
      },
    },
    dataLabels: {
      enabled: true,
      formatter: function (val) {
        return val >= 1000 ? (val / 1000).toFixed(2) + "k" : val;
      },
      offsetY: -20,
    },
    xaxis: {
      categories: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],
      position: "top",
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

  const IncomeEl = document.querySelector("#income-chart");

  setTimeout(() => {
    IncomeEl._chart = new ApexCharts(IncomeEl, incomeConfig);
    IncomeEl._chart.render();
  });

  // Contact List Accordion
  new Accordion("#contacts-accordion", {
    duration: 200,
    openOnInit: [0],
  });

  // Messages Carousel
  const messagesCarousel = document.querySelector("#messages-carousel");
  messagesCarousel._swiper = new Swiper(messagesCarousel, {
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

  // Ongoing Projects Menu
  new Popper(
    "#ongoing-projects-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Contact List Menu
  new Popper(
    "#contact-list-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Client Messages Menu
  new Popper(
    "#client-messages-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Income Menu
  new Popper("#income-menu", ".popper-ref", ".popper-root", dropdownConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
