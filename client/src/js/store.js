export default () => ({
  isDarkModeEnabled: Alpine.$persist(false).as("_x_darkMode_on"),
  isMonochromeModeEnabled: false,
  isSearchbarActive: false,
  isSidebarExpanded: false,
  isRightSidebarExpanded: false,
  currentLocation: currentLocation(),
  init() {
    this.isSidebarExpanded =
      document.querySelector(".sidebar") &&
      document.body.classList.contains("is-sidebar-open") &&
      Alpine.store("breakpoints").xlAndUp;

    Alpine.effect(() => {
      this.isMonochromeModeEnabled
        ? document.body.classList.add("is-monochrome")
        : document.body.classList.remove("is-monochrome");
    });

    Alpine.effect(() => {
      this.isSidebarExpanded
        ? document.body.classList.add("is-sidebar-open")
        : document.body.classList.remove("is-sidebar-open");
    });

    Alpine.effect(() => {
      if (Alpine.store("breakpoints").smAndUp) this.isSearchbarActive = false;
    });

    window.addEventListener('changed:breakpoint', () => {
      if (this.isSidebarExpanded) this.isSidebarExpanded = false;
      if (this.isRightSidebarExpanded) this.isRightSidebarExpanded = false;
    })
  },

  documentBody: {
    ["@load.window"]() {
      const preloader = document.querySelector(".app-preloader");
      if (!preloader) return;
      setTimeout(() => {
        preloader.classList.add(
          "animate-[cubic-bezier(0.4,0,0.2,1)_fade-out_500ms_forwards]"
        );
        setTimeout(() => preloader.remove(), 1000);
      }, 150);
    },
  },
});

function currentLocation() {
  return [location.protocol, "//", location.host, location.pathname].join("");
}
