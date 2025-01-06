const onLoad = () => {
  // Credit Cards Carousel
  const cardsCarousel = document.querySelector("#cards-carousel");
  cardsCarousel_swiper = new Swiper(cardsCarousel, {
    slidesPerView: "auto",
    spaceBetween: 16,
  });

  // History Transactions Chart
  const historyConfig = {
    colors: ["#4C4EE7", "#0EA5E9"],
    series: [
      {
        name: "Sales",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28, 66],
      },
      {
        name: "Profit",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14, 22],
      },
    ],
    chart: {
      height: 330,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    plotOptions: {},
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
    stroke: {
      width: 3,
    },
    markers: {
      size: 5,
      hover: {
        size: 8,
      },
    },
    grid: {
      padding: {
        left: 10,
        right: 0,
        top: -30,
        bottom: -8,
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

  const historyEl = document.querySelector("#history-chart");

  setTimeout(() => {
    historyEl._chart = new ApexCharts(historyEl, historyConfig);
    historyEl._chart.render();
  });

  // Credit Card number
  const cardNumber = document.querySelector("#card-number");
  cardNumber._mask = new Cleave(cardNumber, {
    creditCard: true,
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

  // History Menu
  new Popper("#history-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Send Money Menu
  new Popper("#send-money-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Balance Menu
  new Popper("#balance-menu", ".popper-ref", ".popper-root", dropdownConfig);
};

window.addEventListener("app:mounted", onLoad, { once: true });
