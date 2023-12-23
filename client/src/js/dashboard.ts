import { invoke } from "@tauri-apps/api/tauri";
import { WebviewWindow } from '@tauri-apps/api/window'

import Swiper from "swiper/bundle";
import Drawer from "./components/drawer";
import Popper from "./components/popper";
import Tab from "./components/tab";

export default class Dashboard {
    constructor() {
        const page = document.querySelector("#dashboard-page");
        if (page == null) {
            return;
        }

        console.log("Loading dashboard");

        const mainEl = document.querySelector("main.chat-app");
        const historySlide = document.querySelector("#history-slide") as HTMLElement;
        const chatDetailToggleEl = document.querySelector("#chat-detail-toggle");

        new Swiper((historySlide!), {
            slidesPerView: "auto",
            spaceBetween: 10,
            slidesPerGroup: 3,
        });

        new Popper("#chat-menu", ".popper-ref", ".popper-root", {
            placement: "bottom-end",
            modifiers: [
                {
                    name: "offset",
                    options: {
                        offset: [0, 4],
                    },
                },
            ],
        });

        new Tab("#tab-media");
    }
}