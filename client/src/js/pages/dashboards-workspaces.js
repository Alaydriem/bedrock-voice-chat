const onLoad = () => {
  // Server Traffic Chart
  const trafficConfig = {
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
  };

  const trafficEl = document.querySelector("#traffic-chart");

  setTimeout(() => {
    trafficEl._chart = new ApexCharts(trafficEl, trafficConfig);
    trafficEl._chart.render();
  });

  // CPU Usage Chart
  const cpuConfig = {
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
  };

  const cpuEl = document.querySelector("#cpu-chart");

  setTimeout(() => {
    cpuEl._chart = new ApexCharts(cpuEl, cpuConfig);
    cpuEl._chart.render();
  });

  // Storage Usage Chart
  const storageConfig = {
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
  };

  const storageEl = document.querySelector("#storage-chart");

  setTimeout(() => {
    storageEl._chart = new ApexCharts(storageEl, storageConfig);
    storageEl._chart.render();
  });

  // Memory Usage Chart
  const memoryConfig = {
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
  };

  const memoryEl = document.querySelector("#memory-chart");

  setTimeout(() => {
    memoryEl._chart = new ApexCharts(memoryEl, memoryConfig);
    memoryEl._chart.render();
  });

  // Server Traffic Menu
  new Popper("#traffic-menu", ".popper-ref", ".popper-root", {
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

  // Server Traffic Tabs
  new Tab("#traffic-tab");

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

  // Overview Menu
  new Popper("#monitoring-menu", ".popper-ref", ".popper-root", dropdownConfig);

  // Workspace Menu 1
  new Popper(
    "#workspace-menu-1",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Workspace Menu 2
  new Popper(
    "#workspace-menu-2",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Workspace Menu 3
  new Popper(
    "#workspace-menu-3",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Workspace Menu 4
  new Popper(
    "#workspace-menu-4",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Workspace Menu 5
  new Popper(
    "#workspace-menu-5",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Workspace Menu 6
  new Popper(
    "#workspace-menu-6",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );
};

window.addEventListener("app:mounted", onLoad, { once: true });
