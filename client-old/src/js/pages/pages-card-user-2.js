const onLoad = () => {
  new Popper("#top-header-menu", ".popper-ref", ".popper-root", {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  });

  cardUser1 = {
    colors: ["#6366f1"],
    series: [
      {
        name: "Posts",
        data: [48, 100, 70, 92],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 85,

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
        bottom: 0,
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
  cardUser2 = {
    colors: ["#F000B9"],
    series: [
      {
        name: "Posts",
        data: [54, 77, 43, 69, 12],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 85,

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
        bottom: 0,
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
  cardUser3 = {
    colors: ["#10B981"],
    series: [
      {
        name: "Posts",
        data: [0, 100, 0],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 85,

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
        bottom: 0,
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
  cardUser4 = {
    colors: ["#FF5724"],
    series: [
      {
        name: "Posts",
        data: [0, 20, 10, 30, 20, 50],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 85,

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
        bottom: 0,
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
  cardUser5 = {
    colors: ["#FF9800"],
    series: [
      {
        name: "Posts",
        data: [33, 77, 55, 102, 12],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 85,

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
        bottom: 0,
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

  const postChart1 = document.querySelector("#postChart1");
  const postChart2 = document.querySelector("#postChart2");
  const postChart3 = document.querySelector("#postChart3");
  const postChart4 = document.querySelector("#postChart4");
  const postChart5 = document.querySelector("#postChart5");
  const postChart6 = document.querySelector("#postChart6");
  const postChart7 = document.querySelector("#postChart7");
  const postChart8 = document.querySelector("#postChart8");

  setTimeout(() => {
    postChart1._chart = new ApexCharts(postChart1, cardUser1);
    postChart1._chart.render();
  });

  setTimeout(() => {
    postChart2._chart = new ApexCharts(postChart2, cardUser2);
    postChart2._chart.render();
  });

  setTimeout(() => {
    postChart3._chart = new ApexCharts(postChart3, cardUser3);
    postChart3._chart.render();
  });

  setTimeout(() => {
    postChart4._chart = new ApexCharts(postChart4, cardUser4);
    postChart4._chart.render();
  });
  
  setTimeout(() => {
    postChart5._chart = new ApexCharts(postChart5, cardUser5);
    postChart5._chart.render();
  });

  setTimeout(() => {
    postChart6._chart = new ApexCharts(postChart6, cardUser1);
    postChart6._chart.render();
  });

  setTimeout(() => {
    postChart7._chart = new ApexCharts(postChart7, cardUser2);
    postChart7._chart.render();
  });

  setTimeout(() => {
    postChart8._chart = new ApexCharts(postChart8, cardUser3);
    postChart8._chart.render();
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
