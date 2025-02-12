export const charts = {
  // Demo charts
  demoChart1: {
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
  },
  demoChart2: {
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
  },
  demoChart3: {
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
  },
  demoChart4: {
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
  },
  demoChart5: {
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
  },
  demoChart6: {
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
  },
  demoChart7: {
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
  },
  demoChart8: {
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
  },
  demoChart9: {
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
  },
  demoChart10: {
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
  },

  // User Card Demo
  cardUser1: {
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
  },
  cardUser2: {
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
  },
  cardUser3: {
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
  },
  cardUser4: {
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
  },
  cardUser5: {
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
  },

  // Widget Chart
  dailyVisitWidget: {
    colors: ["#FF5724"],
    series: [
      {
        data: [0, 20, 10, 30, 20, 50],
      },
    ],
    chart: {
      type: "line",
      stacked: false,
      height: 150,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },

      zoom: {
        enabled: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    grid: {
      show: false,
      padding: {
        left: 0,
        right: 0,
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
  },

  // Dashboard charts

  // Interview Dashboard Chart
  interview: {
    colors: ["#0EA5E9"],
    series: [
      {
        name: "Interviews",
        data: [31, 40, 28, 51, 42, 61, 45],
      },
    ],
    chart: {
      height: "220",
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
      width: 2,
    },

    xaxis: {
      type: "datetime",
      categories: [
        "2018-09-19",
        "2018-09-20",
        "2018-09-21",
        "2018-09-22",
        "2018-09-23",
        "2018-09-24",
        "2018-09-25",
      ],
    },
    tooltip: {
      x: {
        format: "dd/MM/yy HH:mm",
      },
    },
    legend: {
      show: false,
    },
  },

  // Task Report Chart
  tasksReport: {
    colors: ["#81E4C0", "#FFC233", "#F000B9"],
    series: [
      {
        name: "Completed",
        data: [31, 40, 28, 51, 42, 109, 100],
      },
      {
        name: "Blocked",
        data: [48, 32, 45, 77, 86, 52, 41],
      },
      {
        name: "Pending",
        data: [75, 64, 55, 62, 34, 62, 91],
      },
    ],
    chart: {
      height: 300,
      type: "area",
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
        top: 0,
        bottom: 0,
      },
    },
    yaxis: {
      labels: {
        offsetX: -12,
        offsetY: 0,
      },
    },
    stroke: {
      curve: "smooth",
      width: "2",
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
      offsetX: -28,
    },
  },

  // Task Status Chart
  tasksStatus: {
    colors: ["#4C4EE7", "#0EA5E9"],
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
      height: 250,
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
        top: 0,
        bottom: 0,
      },
    },
  },

  // Team Efficiency
  teamefficiency: {
    series: [44, 55, 66],
    labels: ["Design", "Development", "Support"],
    chart: {
      type: "donut",
      height: "200px",
    },
    legend: { show: false },
    dataLabels: { enabled: false },
    plotOptions: {
      pie: {
        donut: {
          labels: {
            show: true,
            total: {
              show: true,
              label: "Design",
              color: undefined,
              fontFamily: undefined,
              formatter: function (w) {
                return w.config.series[0];
              },
            },
          },
        },
      },
    },
    theme: {
      monochrome: {
        color: "#5F5AF6",
        enabled: true,
        shadeTo: "light",
        shadeIntensity: 0.65,
      },
    },
  },

  // Earning White Dashboard
  earningWhite: {
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
  },

  // Income Personal Dashboard
  incomePersonal: {
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
  },

  // Courses Timeline
  courseTimeline: {
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
  },

  // Blog Visitors
  blogVisitors: {
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
  },

  // Blog Members
  blogMembers: {
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
  },

  // Authors Statistic
  blogAuthors1: {
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
  },

  blogAuthors2: {
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
  },

  blogAuthors3: {
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
  },

  blogAuthors4: {
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
  },

  blogAuthors5: {
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
  },

  // Teacher Working Hours
  workingHours: {
    colors: ["#0EA5E9"],
    series: [70],
    chart: {
      height: 210,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          margin: 0,
          size: "70%",
        },
        dataLabels: {
          name: {
            show: false,
          },
          value: {
            show: true,
            color: "#333",
            offsetY: 10,
            fontSize: "24px",
            fontWeight: 600,
          },
        },
      },
    },
    grid: {
      show: false,
      padding: {
        left: 0,
        right: 0,
        top: 0,
        bottom: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
  },

  // Doctor Patient Count
  patientCount: {
    colors: ["#0EA5E9", "#F000B9"],
    series: [
      {
        name: "Man",
        data: [28, 45, 35, 50, 32],
      },
      {
        name: "Woman",
        data: [14, 25, 20, 25, 12],
      },
    ],
    chart: {
      height: 210,
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
      categories: ["Jan", "Feb", "Mar", "Apr", "May"],

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
        bottom: -8,
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
  },

  // Employee Sales Growth
  salesGrowth: {
    colors: ["#FF9800"],
    series: [
      {
        name: "Sales Growth",
        data: [35, 20, 45, 30, 55, 27, 45],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 120,
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
        bottom: -15,
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
  },

  // Employee Clients Growth
  clientsGrowth: {
    colors: ["#10B981"],
    series: [
      {
        name: "Clients Growth",
        data: [45, 20, 55, 28, 45, 25, 65],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 120,
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
        bottom: -15,
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
  },

  // Employee Task Status
  employeesTask: {
    colors: ["#0EA5E9"],
    series: [65],
    chart: {
      height: 120,
      width: 120,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          size: "60%",
        },
        dataLabels: {
          name: {
            show: false,
            color: "#fff",
          },
          value: {
            show: true,
            fontSize: "18px",
            offsetY: 4,
          },
        },
      },
    },
    grid: {
      show: false,
      padding: {
        left: -20,
        right: -20,
        top: -20,
        bottom: -23,
      },
    },
    stroke: {
      lineCap: "round",
    },
  },

  // Workspace Server Traffic
  serverTraffic: {
    colors: ["#4C4EE7", "#0EA5E9"],
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
      height: 260,
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
      categories: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"],

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
        left: -8,
        right: -8,
        top: 0,
        bottom: -6,
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
  },

  // Workspace CPU usage
  cpuUsage: {
    colors: ["#0EA5E9"],
    series: [76],
    chart: {
      height: "200px",
      type: "radialBar",
      sparkline: {
        enabled: true,
      },
    },
    plotOptions: {
      radialBar: {
        startAngle: -90,
        endAngle: 90,

        dataLabels: {
          name: {
            show: false,
          },
          value: {
            offsetY: -2,
            fontSize: "18px",
          },
        },
      },
    },
    responsive: [
      {
        breakpoint: 400,
        options: {
          chart: {
            height: "160px",
          },
        },
      },
    ],
    grid: {
      padding: {
        top: 0,
        right: -10,
        bottom: 0,
        left: -10,
      },
    },

    labels: ["Average Results"],
  },

  // Workspace Storage Usage
  storageUsage: {
    colors: ["#4C4EE7"],
    series: [45],
    chart: {
      height: 80,
      width: 50,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          size: "45%",
        },
        dataLabels: {
          name: {
            show: false,
          },
          value: {
            offsetY: 5,
            show: true,
            fontSize: "12px",
          },
        },
      },
    },
    grid: {
      padding: {
        top: -15,
        right: 0,
        bottom: -17,
        left: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
  },

  // Workspace Memory Usage
  memoryUsage: {
    colors: ["#0EA5E9"],
    series: [45],
    chart: {
      height: 80,
      width: 50,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          size: "45%",
        },
        dataLabels: {
          name: {
            show: false,
            color: "#fff",
          },
          value: {
            offsetY: 5,
            show: true,
            fontSize: "12px",
          },
        },
      },
    },
    grid: {
      padding: {
        top: -15,
        right: 0,
        bottom: -17,
        left: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
  },

  // Sales Report
  salesReport: {
    colors: ["#4C4EE7", "#0EA5E9"],
    series: [
      {
        name: "Sales",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28, 45, 35, 50],
      },
      {
        name: "Profit",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14, 25, 20, 25],
      },
    ],
    chart: {
      height: 260,
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
    responsive: [
      {
        breakpoint: 850,
        options: {
          plotOptions: {
            bar: {
              columnWidth: "55%",
            },
          },
        },
      },
    ],
  },

  salesEarning: {
    series: [
      {
        name: "Earning A",
        data: [44, 55, 41, 25, 22],
      },
      {
        name: "Earning 0",
        data: [13, 23, 20, 60, 13],
      },
    ],
    grid: {
      show: false,
      padding: {
        left: -8,
        right: 0,
        bottom: -8,
        top: 0,
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
    chart: {
      type: "bar",
      height: 120,
      toolbar: {
        show: false,
      },
      stacked: true,
      stackType: "100%",
    },
    dataLabels: {
      enabled: false,
    },
    fill: {
      colors: ["#0EA5E9", "#e2e8f0"],
    },
    plotOptions: {
      bar: {
        borderRadius: 2,
        horizontal: false,
        columnWidth: 20,
      },
    },
    legend: {
      show: false,
    },
  },

  salesRating: {
    colors: ["#10b981"],
    series: [85],
    chart: {
      height: 80,
      width: 50,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          size: "45%",
        },
        dataLabels: {
          name: {
            show: false,
          },
          value: {
            offsetY: 5,
            show: true,
            fontSize: "12px",
          },
        },
      },
    },
    grid: {
      padding: {
        top: -15,
        right: 0,
        bottom: -17,
        left: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
  },

  salesCompletedOrders: {
    colors: ["#4C4EE7"],
    series: [45],
    chart: {
      height: 80,
      width: 50,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          size: "45%",
        },
        dataLabels: {
          name: {
            show: false,
          },
          value: {
            offsetY: 5,
            show: true,
            fontSize: "12px",
          },
        },
      },
    },
    grid: {
      padding: {
        top: -15,
        right: 0,
        bottom: -17,
        left: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
  },

  salesOrders: {
    colors: ["#FF9800"],
    series: [
      {
        name: "Orders",
        data: [20, 50, 30, 60, 25, 82],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 140,
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
        bottom: -10,
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
  },

  topSeller1: {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
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
        top: -10,
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
  },

  topSeller2: {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
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
        top: -10,
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
  },

  topSeller3: {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
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
        top: -10,
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
  },

  topSeller4: {
    colors: ["#4467EF"],
    chart: {
      height: 100,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Income",
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
        top: -10,
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
  },

  // Orders Overview
  ordersOverview: {
    colors: ["#4C4EE7", "#26E7A6", "#FF9800"],
    series: [
      {
        name: "Orders",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28],
      },
      {
        name: "Completed Orders",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14],
      },
      {
        name: "Returned Orders",
        data: [4, 5, 6, 5, 2, 5, 3, 6, 3],
      },
    ],
    chart: {
      height: 270,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
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
        bottom: 0,
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
        breakpoint: 850,
        options: {
          plotOptions: {
            bar: {
              columnWidth: "55%",
            },
          },
        },
      },
    ],
  },

  orderBudget: {
    series: [
      {
        name: "Start",
        data: [44, 55, 41, 25, 22, 56],
      },
      {
        name: "End",
        data: [13, 23, 20, 60, 13, 16],
      },
    ],
    grid: {
      show: false,
      padding: {
        left: 0,
        right: 10,
        bottom: -12,
        top: 0,
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
    chart: {
      type: "bar",
      height: 120,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
      stacked: true,
      stackType: "100%",
    },
    dataLabels: {
      enabled: false,
    },
    fill: {
      colors: ["#0EA5E9", "#e2e8f0"],
    },
    plotOptions: {
      bar: {
        borderRadius: 2,
        horizontal: false,
        columnWidth: 24,
      },
    },
    legend: {
      show: false,
    },
  },

  orderIncome: {
    colors: ["#10b981"],
    series: [
      {
        name: "Income",
        data: [20, 50, 30, 60, 33, 75],
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
  },

  orderExpense: {
    colors: ["#FF5724"],
    series: [
      {
        name: "Expense",
        data: [82, 25, 60, 30, 50, 20],
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
  },

  // Banking Dashboard
  historyTransactions: {
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
  },

  historyTransactionsLine: {
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
  },

  // Analytics Dashboard
  analyticsSalesOverview: {
    colors: ["#4C4EE7", "#0EA5E9"],
    series: [
      {
        name: "Sales",
        data: [28, 45, 35, 50, 32, 55, 23, 60, 28, 45, 35, 50],
      },
      {
        name: "Profit",
        data: [14, 25, 20, 25, 12, 20, 15, 20, 14, 25, 20, 25],
      },
    ],
    chart: {
      height: 255,
      type: "bar",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
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
        breakpoint: 850,
        options: {
          plotOptions: {
            bar: {
              columnWidth: "55%",
            },
          },
        },
      },
    ],
  },

  analyticsSalesThisMonth: {
    colors: ["#4467EF"],
    chart: {
      height: 60,
      type: "line",
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
    },
    series: [
      {
        name: "Sales",
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
        top: -20,
        bottom: -10,
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
  },

  analyticsBandwidth: {
    colors: ["#4467EF"],

    series: [
      {
        name: "Traffic",
        data: [
          8107.85, 8128.0, 8122.9, 8165.5, 8340.7, 8423.7, 8423.5, 8514.3,
          8481.85, 8487.7, 8506.9, 8626.2, 8668.95, 8602.3, 8607.55, 8512.9,
          8496.25, 8600.65, 8881.1, 9340.85,
        ],
      },
    ],
    chart: {
      type: "area",
      height: 220,
      parentHeightOffset: 0,
      toolbar: {
        show: false,
      },
      zoom: {
        enabled: false,
      },
    },
    dataLabels: {
      enabled: false,
    },
    stroke: {
      curve: "smooth",
      width: 2,
    },
    grid: {
      padding: {
        left: 0,
        right: 0,
        top: -28,
        bottom: -15,
      },
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
  },

  // Dashboard Travels
  travelAnalytics: {
    colors: ["#4ade80", "#f43f5e", "#a855f7"],
    series: [44, 55, 67],
    chart: {
      height: 250,
      type: "radialBar",
    },
    plotOptions: {
      radialBar: {
        hollow: {
          margin: 10,
          size: "35%",
        },
        track: {
          margin: 10,
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
    grid: {
      padding: {
        top: -20,
        bottom: -20,
        right: 0,
        left: 0,
      },
    },
    stroke: {
      lineCap: "round",
    },
    labels: ["Booked", "Cancelled", "Unconfirmed"],
  },

  travelExpense: {
    colors: ["#0EA5E9"],
    series: [
      {
        name: "Expense",
        data: [82, 25, 60, 30, 50, 20],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 180,
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
  },

  // Dashboard influencer
  influencerActivity: {
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
  },

  // Dashboard Crypto
  cryptoWatchlist1: {
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
  },

  cryptoWatchlist2: {
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
  },

  cryptoWatchlist3: {
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
  },

  cryptoWatchlist4: {
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
  },

  cryptoTransactions: {
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
  },

  cryptoCandlestick: {
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
      height: 364,
      type: "candlestick",
      parentHeightOffset: 0,

      toolbar: {
        show: false,
      },
      events: {
        mounted: (chart) => {
          chart.windowResizeHandler();
        },
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
      padding: {
        left: -12,
        right: -12,
        top: -6,
        bottom: 0,
      },
    },
    tooltip: {
      enabled: true,
    },
    xaxis: {
      type: "category",
      tickAmount: 10,
      labels: {
        formatter: function (val) {
          return dayjs(val).format("HH:mm");
        },
        rotate: 0,
      },
    },
    yaxis: {
      tooltip: {
        enabled: true,
      },
      labels: {
        offsetX: -14,
        formatter: function (val) {
          return val / 1000 + "k";
        },
      },
    },
    responsive: [
      {
        breakpoint: 640,
        options: {
          xaxis: {
            tickAmount: 6,
          },
        },
      },
    ],
  },

  stockMarket1: {
    colors: ["#F7931A"],
    series: [
      {
        name: "Price",
        data: [20, 50, 30, 60, 33, 75],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 100,
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
  },

  stockMarket2: {
    colors: ["#3AC5BC"],
    series: [
      {
        name: "Price",
        data: [654, 820, 102, 540, 154, 614],
      },
    ],
    chart: {
      type: "area",
      stacked: false,
      height: 100,
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
  },

  analyticsPagesViews: {
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
  },

  analyticsSiteOverview: {
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
  },
};
