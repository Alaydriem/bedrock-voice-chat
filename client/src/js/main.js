import { onOpenUrl } from '@tauri-apps/plugin-deep-link';

/**
 * Collapse/Accordion Library
 * @see https://github.com/michu2k/Accordion
 */
import Accordion from "accordion-js";

/**
 * Scrollbar Library
 * @see https://github.com/Grsmto/simplebar
 */
import SimpleBar from "simplebar";

/**
 * Date Utility Library
 * @see https://day.js.org/
 */
import dayjs from "dayjs";

/**
 *  Breakpont Service
 */
import Breakpont from "./services/breakpoint";

/**
 * Darkmode Service
 */
import DarkMode from "./services/darkMode";

/**
 * Monochrome Mode Service
 */
import MonochromeMode from "./services/monochromeMode";

/**
 * Notification Service
 */
import Notification from "./services/notification";

/**
 * Clipboard Service
 */
import Clipboard from "./services/clipboard";

/**
 * Helper Functions
 */
import * as helpers from "./utils/helpers";

/**
 * Popper JS
 * @see https://popper.js.org/
 */
import Popper from "./components/popper";

/**
 * Tab Component
 */
import Tab from "./components/tab";

/**
 * Tab Component
 */
import Modal from "./components/modal";

/**
 * Drawer Component
 */
import Drawer from "./components/drawer";

/**
 * Tooltip Component
 */
import * as tooltip from "./components/tooltip";

/**
 * Application Services
 */
window.$breakpoint = new Breakpont();
window.$darkmode = new DarkMode();
window.$monochromemode = new MonochromeMode();
window.$notification = Notification;
window.$clipboard = Clipboard;

window.helpers = helpers;
window.Popper = Popper;
window.Tab = Tab;
window.Modal = Modal;
window.Drawer = Drawer;

window.Accordion = Accordion;
window.SimpleBar = SimpleBar;
window.dayjs = dayjs;
window.tooltip = tooltip;

const PRELOADER_CLASS = "app-preloader";
const ROOT_APP_ID = "root";

const SIDEBAR_CLASS = "sidebar";
const SIDEBAR_OPEN_CLASS = "is-sidebar-open";
const SIDEBAR_TOGGLE_BTN_CLASS = "sidebar-toggle";
const SIDEBAR_CLOSE_BTN_CLASS = "sidebar-close";
const SIDEBAR_NAV_WRAPPER = "nav-wrapper";
const SIDEBAR_NAV_LINK_CLASS = "nav-link";
const SIDEBAR_NAV_PARENT_CLASS = "nav-parent";

const RIGHT_SIDEBAR_ID = "right-sidebar";
const RIGHT_SIDEBAR_HEADER_CLASS = "right-sidebar-header";

const DARKMODE_TOGGLE_BTN_CLASS = "darkmode-toggle";
const MONOCHROME_TOGGLE_BTN_CLASS = "monochrome-toggle";

const NOTIFICATION_WRAPPER_ID = "notification-wrapper";
const NOTIFICATION_REF_ID = "notification-ref";
const NOTIFICATION_BOX_ID = "notification-box";

const SEARCHBAR_WRAPPER_ID = "searchbar-wrapper";
const SEARCHBAR_REF_ID = "searchbar-ref";
const SEARCHBAR_BOX_ID = "searchbar-box";

const MOBILE_SEARCHBAR_CLASS = "mobile-searchbar";
const MOBILE_SEARCHBAR_SHOW_CLASS = "mobile-searchbar-show";
const MOBILE_SEARCHBAR_HIDE_CLASS = "mobile-searchbar-hide";
const MOBILE_SEARCHBAR_INPUT_CLASS = "mobile-searchbar-input";

const PROFILE_WRAPPER_ID = "profile-wrapper";
const PROFILE_REF_ID = "profile-ref";
const PROFILE_BOX_ID = "profile-box";

const NOTIFICATION_TAB_CLASS = "notification-tab-wrapper";
const SEARCH_TAB_CLASS = "search-tab-wrapper";
const MOBILE_SEARCH_TAB_CLASS = "mobile-search-tab-wrapper";
const RIGHT_SIDEBAR_TAB_CLASS = "right-sidebar-tab-wrapper";

const TABLE_SEARCH_CLASS = "table-search-wrapper";
const TABLE_SEARCH_INPUT_CLASS = "table-search-input";
const TABLE_SEARCH_TOGGLE_CLASS = "table-search-toggle";

await onOpenUrl(async (urls) => {
  await Main.openDeepLink(urls);
});

export default class Main {
  _html = document.documentElement;
  _body = document.body;
  _root = document.querySelector(`#${ROOT_APP_ID}`);

  _sidebar = document.querySelector(`.${SIDEBAR_CLASS}`);
  _mobileSearchbar = document.querySelector(`.${MOBILE_SEARCHBAR_CLASS}`);

  currentLocation = helpers.getCurrentLocation();

  popperNotification = null;
  popperSearchbar = null;
  popperProfile = null;
  sidebarNav = null;
  notificationTab = null;
  rightSidebarTab = null;

  /**
   * The constructor method is a special method of a class
   * for creating and initializing an object instance of that class.
   */
  constructor() {
    this._uiInit();
  }

  /**
   * @param {string[]} urls
   */
  static async openDeepLink(urls) {
    window.App.openDeepLink(urls);
  }

  _uiInit() {
    this._uiInitSidebar();
    this._uiInitSidebarNav();

    this._uiInitDarkModeBtn();
    this._uiInitMonochromeBtn();

    this._uiInitNotification();
    this._uiInitSearchbar();
    this._uiInitMobileSearchbar();

    this._uiInitProfile();

    this._uiInitNotificationTab();
    this._uiInitSearchTab();
    this._uiIniMobiletSearchTab();

    this._uiInitRightSidebar();
    this._uiInitRightSidebarTabs();

    this._uiInitTableSearchbar();
    this._root.classList.remove("cloak");

    this.removeAppPreloader();
  }

  _uiInitSidebar() {
    // checking the existing of the left sidebar
    if (!this._sidebar) return;

    // Closing the sidebar at the breakponts smaller than "XL"
    if (!$breakpoint.xlAndUp) this._uiCloseSidebar();

    // Declaring the toggle buttons for sidebar
    const toggleBtns = document.querySelectorAll(
      `.${SIDEBAR_TOGGLE_BTN_CLASS}`,
    );

    // Declaring the close buttons for sidebar
    const closeBtns = document.querySelectorAll(`.${SIDEBAR_CLOSE_BTN_CLASS}`);

    // Closing the sidebar when breakpoint changed
    window.addEventListener("change:breakpoint", () => {
      if (this._body.classList.contains(SIDEBAR_OPEN_CLASS))
        this._uiCloseSidebar();
    });

    toggleBtns.forEach((node) =>
      node.addEventListener("click", () => this._uiToggleSidebar()),
    );

    closeBtns.forEach((node) =>
      node.addEventListener("click", () => this._uiCloseSidebar()),
    );
  }

  _uiInitSidebarNav() {
    // Select the navigation links
    const navLinks = document.querySelectorAll(`.${SIDEBAR_NAV_LINK_CLASS}`);
    const navParents = document.querySelectorAll(
      `.${SIDEBAR_NAV_PARENT_CLASS}`,
    );

    // checking the existing of the navigation links
    if (!(navLinks.length > 0)) return;

    // Declaring default opened parent active navigation link
    let openOnInit = null;

    // Add index for parent navigation links
    if (navParents) {
      navParents.forEach((node, i) => {
        node.dataset.navParentIndex = i;
      });
    }

    navLinks.forEach((node) => {
      // checking if the HTML element is link
      if (!node.href || !node.dataset.activeClass || !node.dataset.defaultClass)
        return;

      // checking if navigation link have parent
      const parent = node.parentNode.closest(`.${SIDEBAR_NAV_PARENT_CLASS}`);

      // get active and default classes for navigation links from "data-active-class" and "data-default-class"
      const activeClass = node.dataset.activeClass.split(" ");
      const defaultClass = node.dataset.defaultClass.split(" ");

      // cleaning the "href" value of the navigation link
      const href = node.href.split("?")[0].split("#")[0];

      // comparing current link with location
      const isActive = href === this.currentLocation;

      if (isActive) {
        node.classList.add(...activeClass);
        setTimeout(() => node.scrollIntoView({ block: "center" }));
        if (parent) {
          openOnInit = parseInt(parent.dataset.navParentIndex);
        }
      } else {
        node.classList.add(...defaultClass);
      }
    });

    this.sidebarNav = new Accordion(`.${SIDEBAR_NAV_WRAPPER}`, {
      onlyChildNodes: false,
      duration: 200,
      openOnInit: [openOnInit],
    });
  }

  _uiExpandSidebar() {
    this._body.classList.add(SIDEBAR_OPEN_CLASS);
  }

  _uiCloseSidebar() {
    this._body.classList.remove(SIDEBAR_OPEN_CLASS);
  }

  _uiToggleSidebar() {
    this._body.classList.toggle(SIDEBAR_OPEN_CLASS);
  }

  _uiInitDarkModeBtn() {
    const toggleBtns = document.querySelectorAll(
      `.${DARKMODE_TOGGLE_BTN_CLASS}`,
    );

    toggleBtns.forEach((node) => {
      node.addEventListener("click", () => $darkmode.toggle());
    });
  }

  _uiInitMonochromeBtn() {
    const toggleBtns = document.querySelectorAll(
      `.${MONOCHROME_TOGGLE_BTN_CLASS}`,
    );

    toggleBtns.forEach((node) => {
      node.addEventListener("click", () => $monochromemode.toggle());
    });
  }

  _uiInitNotification() {
    if (!document.querySelector(`#${NOTIFICATION_WRAPPER_ID}`)) return;

    const config = {
      placement: "bottom-end",
      modifiers: [
        {
          name: "offset",
          options: {
            offset: [0, 12],
          },
        },
      ],
    };

    this.popperNotification = new Popper(
      `#${NOTIFICATION_WRAPPER_ID}`,
      `#${NOTIFICATION_REF_ID}`,
      `#${NOTIFICATION_BOX_ID}`,
      config,
    );
  }

  _uiInitSearchbar() {
    const inputRef = document.querySelector(`#${SEARCHBAR_REF_ID}`);

    if (!inputRef) return;

    const config = {
      placement: "bottom-end",
      modifiers: [
        {
          name: "offset",
          options: {
            offset: [0, 12],
          },
        },
      ],
    };

    const onToggle = (isActive) => {
      if (isActive) inputRef.classList.replace("w-60", "w-80");
      else inputRef.classList.replace("w-80", "w-60");
    };

    this.popperSearchbar = new Popper(
      `#${SEARCHBAR_WRAPPER_ID}`,
      `#${SEARCHBAR_REF_ID}`,
      `#${SEARCHBAR_BOX_ID}`,
      config,
      "focus",
      onToggle,
    );

    window.addEventListener("change:breakpoint", (evt) => {
      if (!evt.detail.smAndUp) this.popperNotification.closePopper();
    });
  }

  _uiInitMobileSearchbar() {
    if (!this._mobileSearchbar) return;

    const showBtns = document.querySelectorAll(
      `.${MOBILE_SEARCHBAR_SHOW_CLASS}`,
    );
    const hideBtns = document.querySelectorAll(
      `.${MOBILE_SEARCHBAR_HIDE_CLASS}`,
    );

    if (showBtns) {
      showBtns.forEach((node) => {
        node.addEventListener("click", () => {
          if (!$breakpoint.smAndUp) this._uiShowMobileSearchbar();
        });
      });
    }

    if (hideBtns) {
      hideBtns.forEach((node) => {
        node.addEventListener("click", () => this._uiHideMobileSearchbar());
      });
    }

    window.addEventListener("change:breakpoint", (evt) => {
      if (
        evt.detail.smAndUp &&
        !this._mobileSearchbar.classList.contains("hidden")
      )
        this._uiHideMobileSearchbar();
    });
  }

  _uiShowMobileSearchbar() {
    this._mobileSearchbar.classList.replace("hidden", "flex");
    const input = document.querySelector(`.${MOBILE_SEARCHBAR_INPUT_CLASS}`);

    setTimeout(() => input.focus());
  }

  _uiHideMobileSearchbar() {
    helpers.leaveAnimation(this._mobileSearchbar, () => {
      this._mobileSearchbar.classList.replace("flex", "hidden");
    });
  }

  _uiInitProfile() {
    if (!document.querySelector(`#${PROFILE_WRAPPER_ID}`)) return;

    const config = {
      placement: "right-end",
      modifiers: [
        {
          name: "offset",
          options: {
            offset: [0, 12],
          },
        },
      ],
    };

    this.popperProfile = new Popper(
      `#${PROFILE_WRAPPER_ID}`,
      `#${PROFILE_REF_ID}`,
      `#${PROFILE_BOX_ID}`,
      config,
    );
  }

  _uiInitNotificationTab() {
    const tabWrapper = document.querySelector(`.${NOTIFICATION_TAB_CLASS}`);
    if (tabWrapper) {
      this.notificationTab = new Tab(tabWrapper);
    }
  }

  _uiInitSearchTab() {
    const tabWrapper = document.querySelector(`.${SEARCH_TAB_CLASS}`);
    if (tabWrapper) {
      new Tab(tabWrapper);
    }
  }

  _uiIniMobiletSearchTab() {
    const tabWrapper = document.querySelector(`.${MOBILE_SEARCH_TAB_CLASS}`);
    if (tabWrapper) {
      new Tab(tabWrapper);
    }
  }

  _uiInitRightSidebar() {
    if (!document.querySelector(`#${RIGHT_SIDEBAR_ID}`)) return;

    new Drawer(`#${RIGHT_SIDEBAR_ID}`);
  }

  _uiInitRightSidebarTabs() {
    const tabWrapper = document.querySelector(`.${RIGHT_SIDEBAR_TAB_CLASS}`);
    const header = document.querySelectorAll(`.${RIGHT_SIDEBAR_HEADER_CLASS}`);

    const onChange = (id) => {
      header.forEach((node) => {
        if (node.dataset.header !== id) node.classList.add("hidden");
        else node.classList.remove("hidden");
      });
    };

    if (tabWrapper) {
      this.rightSidebarTab = new Tab(tabWrapper, onChange);
    }

    header.forEach((node) => {
      if (node.dataset.header !== this.rightSidebarTab.aciveTab)
        node.classList.add("hidden");
    });
  }

  _uiInitTableSearchbar() {
    const wrapper = document.querySelectorAll(`.${TABLE_SEARCH_CLASS}`);

    if (wrapper) {
      wrapper.forEach((node) => {
        const input = node.querySelector(`.${TABLE_SEARCH_INPUT_CLASS}`);
        const toggle = node.querySelector(`.${TABLE_SEARCH_TOGGLE_CLASS}`);

        input.isActive = false;

        toggle.addEventListener("click", () =>
          this._uiToggleTableSearchbar(input),
        );
      });
    }
  }

  _uiToggleTableSearchbar(input) {
    if (input.isActive) {
      input.classList.remove("w-32");
      input.classList.remove("lg:w-48");
      input.classList.add("w-0");
      input.isActive = false;
    } else {
      input.classList.remove("w-0");
      input.classList.add("w-32");
      input.classList.add("lg:w-48");
      setTimeout(() => input.focus());
      input.isActive = true;
    }
  }

  removeAppPreloader() {
    const preloader = document.querySelector(`.${PRELOADER_CLASS}`);

    if (!preloader) return;

    setTimeout(() => {
      preloader.classList.add(
        "animate-[var(--ease-in-out)_fade-out_500ms_forwards]",
      );
      setTimeout(() => preloader.remove(), 1000);
    }, 300);
  }
}
