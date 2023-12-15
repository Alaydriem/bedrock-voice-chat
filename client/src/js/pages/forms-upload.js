const onLoad = () => {
  // Basic Filepond
  const filepond1 = document.querySelector("#filepond1");
  filepond1._filepond = FilePond.create(filepond1);

  // Filled Filepond
  const filepond2 = document.querySelector("#filepond2");
  filepond2._filepond = FilePond.create(filepond2);

  // Filled & Bordered
  const filepond3 = document.querySelector("#filepond3");
  filepond3._filepond = FilePond.create(filepond3);

  // Two Grid
  const filepond4 = document.querySelector("#filepond4");
  filepond4._filepond = FilePond.create(filepond4);

  // Three Grid
  const filepond5 = document.querySelector("#filepond5");
  filepond5._filepond = FilePond.create(filepond5);

  // Four Grid
  const filepond6 = document.querySelector("#filepond6");
  filepond6._filepond = FilePond.create(filepond6);

  // Circle Filepond
  const config7 = {
    stylePanelAspectRatio: "1:1",
    stylePanelLayout: "compact circle",
    labelIdle: `<svg xmlns='http://www.w3.org/2000/svg' class='h-8 w-8' fill='none' viewbox='0 0 24 24' stroke='currentColor'>
      <path stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12'/>
    </svg>`,
  };

  const filepond7 = document.querySelector("#filepond7");
  filepond7._filepond = FilePond.create(filepond7, config7);

  const config8 = {
    stylePanelAspectRatio: "1:1",
    stylePanelLayout: "compact circle",
    labelIdle: `<svg xmlns='http://www.w3.org/2000/svg' class='h-8 w-8' fill='none' viewbox='0 0 24 24' stroke='currentColor'>
      <path stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12'/>
    </svg>`,
  };

  const filepond8 = document.querySelector("#filepond8");
  filepond8._filepond = FilePond.create(filepond8, config8);

  const config9 = {
    stylePanelAspectRatio: "1:1",
    stylePanelLayout: "compact circle",
    labelIdle: `<svg xmlns='http://www.w3.org/2000/svg' class='h-8 w-8' fill='none' viewbox='0 0 24 24' stroke='currentColor'>
      <path stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12'/>
    </svg>`,
  };

  const filepond9 = document.querySelector("#filepond9");
  filepond9._filepond = FilePond.create(filepond9, config9);
};

window.addEventListener("app:mounted", onLoad, { once: true });
