const onLoad = () => {
  // Credit Card number
  const cardNumber = document.querySelector("#card-number");
  cardNumber._mask = new Cleave(cardNumber, {
    creditCard: true,
  });

  // Credit Cards Carousel
  const cardsCarousel = document.querySelector("#cards-carousel");
  cardsCarousel_swiper = new Swiper(cardsCarousel, { effect: "cards" });

  // History Transactions Chart
  const historyConfig ={
    colors: ["#FF9800", "#0EA5E9"],
    series: [
      {
        name: "High",
        data: [28, 45, 35, 50, 32, 55, 23, 60],
      },
      {
        name: "Low",
        data: [14, 25, 20, 25, 12, 20, 15, 20],
      },
    ],
    chart: {
      parentHeightOffset: 0,
      height: 290,
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
    grid: {
      padding: {
        left: 10,
        right: 0,
        top: -10,
        bottom: 0,
      },
    },
  };

  const historyEl = document.querySelector("#history-chart");

  setTimeout(() => {
    historyEl._chart = new ApexCharts(historyEl, historyConfig);
    historyEl._chart.render();
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
};

window.addEventListener("app:mounted", onLoad, { once: true });
