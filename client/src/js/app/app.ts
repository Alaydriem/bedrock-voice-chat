import Alpine from "alpinejs";

// AlpineJS Plugins
import persist from "@alpinejs/persist"; // @see https://alpinejs.dev/plugins/persist
import collapse from "@alpinejs/collapse"; // @see https://alpinejs.dev/plugins/collapse
import intersect from "@alpinejs/intersect"; // @see https://alpinejs.dev/plugins/intersect

// Third Party Libraries

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
 * Carousel Library
 * @see https://swiperjs.com/
 */
import Swiper from "swiper/bundle";

/**
 * Drag & Drop Library
 * @see https://github.com/SortableJS/Sortable
 */
import Sortable from "sortablejs";

/**
 * Chart Library
 * @see https://apexcharts.com/
 */
import ApexCharts from "apexcharts";

/**
 * Table Library
 * @see https://gridjs.io/
 */
import * as Gridjs from "gridjs";

//  Forms Libraries
import "@caneara/iodine"; // @see https://github.com/caneara/iodine
import * as FilePond from "filepond"; // @see https://pqina.nl/filepond/
import FilePondPluginImagePreview from "filepond-plugin-image-preview"; // @see https://pqina.nl/filepond/docs/api/plugins/image-preview/
import Quill from "quill"; // @see https://quilljs.com/
import flatpickr from "flatpickr"; // @see https://flatpickr.js.org/
import Tom from "tom-select/dist/js/tom-select.complete.min"; // @see https://tom-select.js.org/

// Helper Functions
import * as helpers from "../utils/helpers";

// Pages Scripts
import * as pages from "../pages";

// Global Store
import store from "../store";

// Breakpoints Store
import breakpoints from "../utils/breakpoints";

// Alpine Components
import usePopper from "../components/usePopper";
import accordionItem from "../components/accordionItem";
import navLink from "../components/navLink";

// Alpine Directives
import tooltip from "../directives/tooltip";
import inputMask from "../directives/inputMask";

// Alpine Magic Functions
import notification from "../magics/notification";
import clipboard from "../magics/clipboard";

declare global {
    interface Window {
        Alpine: any;
        ApexCharts: any,
        helpers: any;
        pages: any;
        FilePond: any;
        dayjs: any;
        SimpleBar: any;
        Swiper: any;
        Sortable: any;
        Gridjs: any;
        flatpickr: any;
        Quill: any;
        Tom: any;
        loadedAlpinePlugins: Set<string>;
    }
}

if (!window.loadedAlpinePlugins) {
    window.loadedAlpinePlugins = new Set<string>();
}

// Function to load a plugin if it hasn't been loaded
function loadAlpinePlugin(pluginName: string, plugin: any) {
    if (!window.loadedAlpinePlugins.has(pluginName)) {
        Alpine.plugin(plugin);
        window.loadedAlpinePlugins.add(pluginName);
        console.log(`Alpine.js plugin "${pluginName}" loaded.`);
    } else {
        console.log(`Alpine.js plugin "${pluginName}" is already loaded.`);
    }
}


export default class App {
    constructor() {
        document.documentElement.classList.add("dark");
        FilePond.registerPlugin(FilePondPluginImagePreview);

        window.dayjs = dayjs;
        window.SimpleBar = SimpleBar;
        window.Swiper = Swiper;
        window.Sortable = Sortable;
        window.ApexCharts = ApexCharts;
        window.Gridjs = Gridjs;
        window.FilePond = FilePond;
        window.flatpickr = flatpickr;
        window.Quill = Quill;
        window.Tom = Tom;

        window.Alpine = Alpine;
        window.helpers = helpers;
        window.pages = pages;

        loadAlpinePlugin("persist", persist);
        loadAlpinePlugin("collapse", collapse);
        loadAlpinePlugin("intersect", intersect);

        Alpine.directive("tooltip", tooltip);
        Alpine.directive("input-mask", inputMask);

        Alpine.magic("notification", () => notification);
        Alpine.magic("clipboard", () => clipboard);

        Alpine.store("breakpoints", breakpoints);
        Alpine.store("global", store());

        Alpine.data("usePopper", usePopper);
        Alpine.data("accordionItem", accordionItem);
        Alpine.data("navLink", navLink);

        Alpine.start();

        const preloader = document.querySelector(".app-preloader");
        if (preloader) {
            setTimeout(() => {
            preloader.classList.add(
                "animate-[cubic-bezier(0.4,0,0.2,1)_fade-out_500ms_forwards]"
            );
            setTimeout(() => preloader.remove(), 1000);
            }, 150);
        }
    }
}