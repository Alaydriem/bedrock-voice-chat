const onLoad = () => {
  // Courses Carousel
  const coursesCarousel = document.querySelector("#courses-carousel");
  coursesCarousel._swiper = new Swiper(coursesCarousel, {
    slidesPerView: "auto",
    spaceBetween: 18,
  });

  // Courses Timeline Chart
  const timelineConfig = {
    series: [
      {
        data: [
          {
            x: "Analysis",
            y: [
              new Date("2019-02-27").getTime(),
              new Date("2019-03-04").getTime(),
            ],
            fillColor: "#008FFB",
          },
          {
            x: "Design",
            y: [
              new Date("2019-03-04").getTime(),
              new Date("2019-03-09").getTime(),
            ],
            fillColor: "#00E396",
          },
          {
            x: "Coding",
            y: [
              new Date("2019-03-07").getTime(),
              new Date("2019-03-10").getTime(),
            ],
            fillColor: "#775DD0",
          },
          {
            x: "Testing",
            y: [
              new Date("2019-03-08").getTime(),
              new Date("2019-03-12").getTime(),
            ],
            fillColor: "#FEB019",
          },
          {
            x: "Deployment",
            y: [
              new Date("2019-03-12").getTime(),
              new Date("2019-03-17").getTime(),
            ],
            fillColor: "#FF4560",
          },
        ],
      },
    ],
    chart: {
      type: "rangeBar",
      height: "200px",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    grid: {
      padding: {
        top: -16,
        bottom: 0,
      },
    },
    plotOptions: {
      bar: {
        horizontal: true,
        distributed: true,
        dataLabels: {
          hideOverflowingLabels: false,
        },
      },
    },
    dataLabels: {
      enabled: false,
    },
    xaxis: {
      type: "datetime",
    },
    yaxis: {
      show: false,
    },
  };

  const timelineEl = document.querySelector("#timeline-chart");

  setTimeout(() => {
    timelineEl._chart = new ApexCharts(timelineEl, timelineConfig);
    timelineEl._chart.render();
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

  // Timeline Menu
  new Popper("#timeline-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Group Lessons Menu
  new Popper(
    "#group-lessons-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
  // Completed Course Menu
  new Popper(
    "#completed-course-menu",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
