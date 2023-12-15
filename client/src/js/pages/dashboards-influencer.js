const onLoad = () => {
  // Influencer Chart
  const influencerConfig = {
    colors: ["#a855f7"],
    series: [
      {
        name: "Sales",
        data: [200, 100, 300, 200, 400, 300, 500],
      },
    ],
    chart: {
      height: 268,
      type: "line",
      parentHeightOffset: 0,

      toolbar: {
        show: false,
      },
      dropShadow: {
        enabled: true,
        color: "#1E202C",
        top: 18,
        left: 6,
        blur: 8,
        opacity: 0.1,
      },
    },
    stroke: {
      width: 5,
      curve: "smooth",
    },
    xaxis: {
      type: "datetime",
      categories: [
        "1/11/2000",
        "2/11/2000",
        "3/11/2000",
        "4/11/2000",
        "5/11/2000",
        "6/11/2000",
        "7/11/2000",
      ],
      tickAmount: 10,
      labels: {
        formatter: function (value, timestamp, opts) {
          return opts.dateFormatter(new Date(timestamp), "dd MMM");
        },
      },
    },
    yaxis: {
      labels: {
        offsetX: -12,
        offsetY: 0,
      },
    },
    fill: {
      type: "gradient",
      gradient: {
        shade: "dark",
        gradientToColors: ["#86efac"],
        shadeIntensity: 1,
        type: "horizontal",
        opacityFrom: 1,
        opacityTo: 0.95,
        stops: [0, 100, 0, 100],
      },
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
      },
    },
  };

  const influencerEl = document.querySelector("#acivity-chart");

  setTimeout(() => {
    influencerEl._chart = new ApexCharts(influencerEl, influencerConfig);
    influencerEl._chart.render();
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
