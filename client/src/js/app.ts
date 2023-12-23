import Main from "./main";
import Login from "./login";
import Dashboard from "./dashboard";

import Swiper from "swiper/bundle";
import Sortable from 'sortablejs';
import ApexCharts from "apexcharts";
import * as Gridjs from "gridjs";
import Popper from "./components/popper";
import Modal from "./components/modal";
import Drawer from "./components/drawer";
import Tab from "./components/tab";

import Accordion from "accordion-js";
import SimpleBar from "simplebar";
import dayjs from "dayjs";
import Breakpont from "./services/breakpoint";
import DarkMode from "./services/darkMode";
import MonochromeMode from "./services/monochromeMode";
import Notification from "./services/notification";
import Clipboard from "./services/clipboard";
import * as helpers from "./utils/helpers";
import * as tooltip from "./components/tooltip";

declare global {
  interface Window {
    App: Application,
    Swiper: Swiper,
    Sortable: Sortable,
    ApexCharts: ApexCharts,
    Gridjs: Gridjs,
    Popper: Popper,
    $breakpoint: any,
    $darkmode: any,
    $monochromemode: any,
    $notification: any,
    $clipboard: any,
    helpers: any,
    Modal: Modal,
    Drawer: Drawer,
    Tab: Tab,
    Accordion: Accordion,
    SimpleBar: SimpleBar
  }
}

export default class Application extends Main {
  constructor() {
    super();
    new Login();
    new Dashboard();
  }
}

window.addEventListener("DOMContentLoaded", () => {
  window.App = new Application();
  window.dispatchEvent(new CustomEvent("app:mounted"));
});