const onLoad = () => {
  // Demo charts
  const demoChart1 = {
    colors: ["#34d399", "#ffba1a"],
    series: [
      {
        name: "Series1",
        data: [31, 40, 28, 51, 42, 109, 100],
      },
      {
        name: "Series2",
        data: [11, 32, 45, 32, 34, 52, 41],
      },
    ],
    chart: {
      height: 350,
      type: "area",
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    stroke: {
      curve: "smooth",
    },
    xaxis: {
      type: "datetime",
      categories: [
        "2018-09-19T00:00:00.000Z",
        "2018-09-19T01:30:00.000Z",
        "2018-09-19T02:30:00.000Z",
        "2018-09-19T03:30:00.000Z",
        "2018-09-19T04:30:00.000Z",
        "2018-09-19T05:30:00.000Z",
        "2018-09-19T06:30:00.000Z",
      ],
    },
    tooltip: {
      x: {
        format: "dd/MM/yy HH:mm",
      },
    },
    legend: {
      position: "top",
      horizontalAlign: "left",
      fontSize: "14px",
    },
  };
  const demoChart2 = {
    colors: ["#6366f1", "#ffba1a"],
    series: [
      {
        name: "PRODUCT B",
        data: [45, 75, 50, 70, 85, 90, 70, 62],
      },
      {
        name: "PRODUCT A",
        data: [30, 16, 27, 30, 55, 60, 48, 43],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 350,
      toolbar: {
        show: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    markers: {
      size: 0,
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
    tooltip: {
      shared: true,
    },
    legend: {
      position: "top",
      horizontalAlign: "right",
      offsetX: -10,
    },
    grid: {
      xaxis: {
        lines: {
          show: true,
        },
      },
      yaxis: {
        lines: {
          show: true,
        },
      },
    },
  };
  const demoChart3 = {
    colors: ["#ff9800", "#6366f1"],
    series: [
      {
        name: "Net Profit",
        data: [44, 55, 57, 56, 61, 58, 63],
      },
      {
        name: "Revenue",
        data: [76, 85, 101, 98, 87, 105, 91],
      },
    ],
    chart: {
      type: "bar",
      height: 350,
      toolbar: {
        show: false,
      },
    },
    plotOptions: {
      bar: {
        horizontal: true,
        columnWidth: "55%",
        borderRadius: 5,
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
    xaxis: {
      categories: ["Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],
    },
    fill: {
      opacity: 1,
    },
    tooltip: {
      y: {
        formatter: function (val) {
          return "$ " + val + " thousands";
        },
      },
    },
    legend: {
      position: "top",
      horizontalAlign: "right",
      fontSize: "14px",
      markers: {
        radius: 12,
      },
    },
  };
  const demoChart4 = {
    colors: ["#a855f7"],
    series: [
      {
        name: "Sales",
        data: [14, 13, 10, 9, 19, 22, 25],
      },
    ],
    chart: {
      height: 350,
      type: "line",
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
      width: 8,
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
    yaxis: {
      min: -10,
      max: 40,
    },
  };
  const demoChart5 = {
    series: [
      {
        name: "candle",
        data: [
          {
            x: new Date(1538778600000),
            y: [6629.81, 6650.5, 6623.04, 6633.33],
          },
          {
            x: new Date(1538780400000),
            y: [6632.01, 6643.59, 6620, 6630.11],
          },
          {
            x: new Date(1538782200000),
            y: [6630.71, 6648.95, 6623.34, 6635.65],
          },
          {
            x: new Date(1538784000000),
            y: [6635.65, 6651, 6629.67, 6638.24],
          },
          {
            x: new Date(1538785800000),
            y: [6638.24, 6640, 6620, 6624.47],
          },
          {
            x: new Date(1538787600000),
            y: [6624.53, 6636.03, 6621.68, 6624.31],
          },
          {
            x: new Date(1538789400000),
            y: [6624.61, 6632.2, 6617, 6626.02],
          },
          {
            x: new Date(1538791200000),
            y: [6627, 6627.62, 6584.22, 6603.02],
          },
          {
            x: new Date(1538793000000),
            y: [6605, 6608.03, 6598.95, 6604.01],
          },
          {
            x: new Date(1538794800000),
            y: [6604.5, 6614.4, 6602.26, 6608.02],
          },
          {
            x: new Date(1538796600000),
            y: [6608.02, 6610.68, 6601.99, 6608.91],
          },
          {
            x: new Date(1538798400000),
            y: [6608.91, 6618.99, 6608.01, 6612],
          },
          {
            x: new Date(1538800200000),
            y: [6612, 6615.13, 6605.09, 6612],
          },
          {
            x: new Date(1538802000000),
            y: [6612, 6624.12, 6608.43, 6622.95],
          },
          {
            x: new Date(1538803800000),
            y: [6623.91, 6623.91, 6615, 6615.67],
          },
          {
            x: new Date(1538805600000),
            y: [6618.69, 6618.74, 6610, 6610.4],
          },
          {
            x: new Date(1538807400000),
            y: [6611, 6622.78, 6610.4, 6614.9],
          },
          {
            x: new Date(1538809200000),
            y: [6614.9, 6626.2, 6613.33, 6623.45],
          },
          {
            x: new Date(1538811000000),
            y: [6623.48, 6627, 6618.38, 6620.35],
          },
          {
            x: new Date(1538812800000),
            y: [6619.43, 6620.35, 6610.05, 6615.53],
          },
          {
            x: new Date(1538814600000),
            y: [6615.53, 6617.93, 6610, 6615.19],
          },
          {
            x: new Date(1538816400000),
            y: [6615.19, 6621.6, 6608.2, 6620],
          },
          {
            x: new Date(1538818200000),
            y: [6619.54, 6625.17, 6614.15, 6620],
          },
          {
            x: new Date(1538820000000),
            y: [6620.33, 6634.15, 6617.24, 6624.61],
          },
          {
            x: new Date(1538821800000),
            y: [6625.95, 6626, 6611.66, 6617.58],
          },
          {
            x: new Date(1538823600000),
            y: [6619, 6625.97, 6595.27, 6598.86],
          },
        ],
      },
    ],
    chart: {
      height: 350,
      type: "candlestick",
      toolbar: {
        show: false,
      },
    },
    grid: {
      xaxis: {
        lines: {
          show: true,
        },
      },
      yaxis: {
        lines: {
          show: true,
        },
      },
    },
    tooltip: {
      enabled: true,
    },
    xaxis: {
      type: "category",
      labels: {
        formatter: function (val) {
          return dayjs(val).format("MMM DD HH:mm");
        },
      },
    },
    yaxis: {
      tooltip: {
        enabled: true,
      },
    },
  };
  const demoChart6 = {
    series: [
      {
        name: "PRODUCT A",
        data: [44, 55, 41, 67, 22, 43, 21, 49],
      },
      {
        name: "PRODUCT B",
        data: [13, 23, 20, 8, 13, 27, 33, 12],
      },
      {
        name: "PRODUCT C",
        data: [11, 17, 15, 15, 21, 14, 15, 13],
      },
    ],
    chart: {
      type: "bar",
      height: 350,
      stacked: true,
      stackType: "100%",
      toolbar: {
        show: false,
      },
    },
    plotOptions: {
      bar: {
        borderRadius: 15,
      },
    },
    xaxis: {
      categories: [
        "2011 Q1",
        "2011 Q2",
        "2011 Q3",
        "2011 Q4",
        "2012 Q1",
        "2012 Q2",
        "2012 Q3",
        "2012 Q4",
      ],
    },
    fill: {
      opacity: 1,
    },
    legend: {
      position: "top",
      horizontalAlign: "left",
      fontSize: "14px",
      markers: {
        radius: 12,
      },
    },
  };
  const demoChart7 = {
    series: [
      {
        name: "SAMPLE A",
        data: [
          [16.4, 5.4],
          [21.7, 2],
          [27.1, 2.3],
          [16.4, 0],
          [13.6, 3.7],
          [10.9, 5.2],
          [16.4, 6.5],
          [10.9, 0],
          [24.5, 7.1],
          [10.9, 0],
          [8.1, 4.7],
        ],
      },
      {
        name: "SAMPLE B",
        data: [
          [36.4, 13.4],
          [1.6, 10],
          [9.9, 2],
          [7.1, 15],
          [1.4, 0],
          [3.6, 13.7],
          [1.9, 15.2],
          [6.4, 16.5],
          [0.9, 10],
          [4.5, 17.1],
          [10.9, 10],
        ],
      },
      {
        name: "SAMPLE C",
        data: [
          [21.7, 3],
          [32.6, 3],
          [27.1, 4],
          [29.6, 6],
          [31.6, 8],
          [21.6, 5],
          [20.9, 4],
          [22.4, 0],
          [32.6, 10.3],
          [29.7, 20.8],
          [24.5, 0.8],
        ],
      },
    ],
    chart: {
      height: 350,
      type: "scatter",
      zoom: {
        enabled: true,
        type: "xy",
      },
      toolbar: {
        show: false,
      },
    },
    xaxis: {
      tickAmount: 10,
      labels: {
        formatter: function (val) {
          return parseFloat(val).toFixed(1);
        },
      },
    },
    yaxis: {
      tickAmount: 7,
    },
    legend: {
      position: "bottom",
      horizontalAlign: "left",
      fontSize: "14px",
    },
  };
  const demoChart8 = {
    colors: ["#4ade80", "#f43f5e", "#a855f7"],
    series: [44, 55, 67],
    chart: {
      height: 350,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          margin: 15,
          size: "35%",
        },
        track: {
          margin: 15,
        },
        dataLabels: {
          name: {
            fontSize: "22px",
          },
          value: {
            fontSize: "16px",
          },
          total: {
            show: true,
            label: "Total",
            formatter: function (w) {
              return w.config.series.reduce((s, v) => s + v);
            },
          },
        },
      },
    },
    stroke: {
      lineCap: "round",
    },
    labels: ["Apples", "Oranges", "Bananas"],
  };
  const demoChart9 = {
    series: [
      {
        name: "Series 1",
        data: [80, 50, 30, 40, 100, 20],
      },
      {
        name: "Series 2",
        data: [20, 30, 40, 80, 20, 80],
      },
      {
        name: "Series 3",
        data: [44, 76, 78, 13, 43, 10],
      },
    ],
    chart: {
      height: 350,
      type: "radar",
      dropShadow: {
        enabled: true,
        blur: 1,
        left: 1,
        top: 1,
      },
    },
    grid: {
      xaxis: {
        lines: {
          show: false,
        },
      },
      yaxis: {
        lines: {
          show: false,
        },
      },
    },
    stroke: {
      width: 2,
    },
    fill: {
      opacity: 0.1,
    },
    markers: {
      size: 0,
    },
    xaxis: {
      categories: ["2011", "2012", "2013", "2014", "2015", "2016"],
    },
  };
  const demoChart10 = {
    series: [42, 47, 52, 58, 65],
    chart: {
      width: 380,
      type: "polarArea",
      animations: {
        enabled: false,
      },
    },
    labels: ["Rose A", "Rose B", "Rose C", "Rose D", "Rose E"],
    fill: {
      opacity: 1,
    },
    stroke: {
      width: 1,
      colors: undefined,
    },
    yaxis: {
      show: false,
    },
    legend: {
      position: "bottom",
      horizontalAlign: "center",
    },
    plotOptions: {
      polarArea: {
        rings: {
          strokeWidth: 0,
        },
        spokes: {
          strokeWidth: 0,
        },
      },
    },
    theme: {
      monochrome: {
        color: "#F000B9",
        enabled: true,
        shadeTo: "light",
        shadeIntensity: 0.6,
      },
    },
  };

  const demo1 = document.querySelector("#demoChart1");
  const demo2 = document.querySelector("#demoChart2");
  const demo3 = document.querySelector("#demoChart3");
  const demo4 = document.querySelector("#demoChart4");
  const demo5 = document.querySelector("#demoChart5");
  const demo6 = document.querySelector("#demoChart6");
  const demo7 = document.querySelector("#demoChart7");
  const demo8 = document.querySelector("#demoChart8");
  const demo9 = document.querySelector("#demoChart9");
  const demo10 = document.querySelector("#demoChart10");

  setTimeout(() => {
    demo1._chart = new ApexCharts(demo1, demoChart1);
    demo1._chart.render();
  });

  setTimeout(() => {
    demo2._chart = new ApexCharts(demo2, demoChart2);
    demo2._chart.render();
  });

  setTimeout(() => {
    demo3._chart = new ApexCharts(demo3, demoChart3);
    demo3._chart.render();
  });
  
  setTimeout(() => {
    demo4._chart = new ApexCharts(demo4, demoChart4);
    demo4._chart.render();
  });

  setTimeout(() => {
    demo5._chart = new ApexCharts(demo5, demoChart5);
    demo5._chart.render();
  });

  setTimeout(() => {
    demo6._chart = new ApexCharts(demo6, demoChart6);
    demo6._chart.render();
  });

  setTimeout(() => {
    demo7._chart = new ApexCharts(demo7, demoChart7);
    demo7._chart.render();
  });

  setTimeout(() => {
    demo8._chart = new ApexCharts(demo8, demoChart8);
    demo8._chart.render();
  });

  setTimeout(() => {
    demo9._chart = new ApexCharts(demo9, demoChart9);
    demo9._chart.render();
  });

  setTimeout(() => {
    demo10._chart = new ApexCharts(demo10, demoChart10);
    demo10._chart.render();
  });
};

window.addEventListener("app:mounted", onLoad, { once: true });
