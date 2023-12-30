import { invoke } from "@tauri-apps/api/tauri";
import { displayCachedImage } from "./utils/imgCache";

import Swiper from "swiper/bundle";
import Drawer from "./components/drawer";
import Popper from "./components/popper";
import Tab from "./components/tab";

import { StreamType } from "./bindings/StreamType";

export default class Dashboard {
  constructor() {
    const page = document.querySelector("#dashboard-page");
    if (page == null) {
      return;
    }

    invoke("input_stream", { s: "1" });
    invoke("output_stream", { s: "1" });

    setTimeout(() => {
      // Test replacing the stream
      invoke("input_stream", { s: "2" });
      invoke("output_stream", { s: "2" });
    }, 5000);

    setTimeout(() => {
      // Test killing the stream
      invoke("stop_stream", { st: "OutputStream" as StreamType });
    }, 6000);

    // Load the players gamer picture
    const profilePicture = document.querySelector(
      "#profile-wrapper #profile-ref img.rounded-full",
    );
    const settingsFlyoverPicture = document.querySelector(
      "#profile-box .avatar img.rounded-full",
    );
    let gamerpic = localStorage.getItem("gamerpic");
    if (gamerpic == null) {
      invoke("get_credential", { key: "gamerpic" })
        .then((result: any) => {
          let raw_image = atob(result);
          displayCachedImage(raw_image, "profile.png").then((image) => {
            let gamerpic = "data:image/png;base64," + image;
            localStorage.setItem("gamerpic", gamerpic);
            profilePicture?.setAttribute("src", gamerpic);
            settingsFlyoverPicture?.setAttribute("src", gamerpic);
          });
        })
        .catch((_) => {
          invoke("logout").then(() => {
            window.location.href = "index.html";
          });
        });
    } else {
      profilePicture?.setAttribute("src", gamerpic);
      settingsFlyoverPicture?.setAttribute("src", gamerpic);
    }

    // Load the players gamertag
    const gt = document.querySelectorAll("#user-gamertag");
    let gamertag = localStorage.getItem("gamertag");
    if (gamertag == null) {
      invoke("get_credential", { key: "gamertag" }).then((result: any) => {
        localStorage.setItem("gamertag", result);
        gt.forEach((el) => {
          el.innerHTML = result;
        });
      });
    } else {
      gt.forEach((el) => {
        el.innerHTML = gamertag as string;
      });
    }

    // Handle de-authentication
    let logoutButton = document.querySelector("#logout-button");
    logoutButton?.addEventListener("click", () => {
      invoke("logout").then(() => {
        window.location.href = "index.html";
      });
    });

    // Settings buttons
    let settingsButtons = document.querySelectorAll("#settings-link");
    settingsButtons.forEach((el) => {
      el.addEventListener("click", (e) => {
        e.preventDefault();
        console.log("click");
        window.location.href = el.getAttribute("href") as string;
      });
    });
    const mainEl = document.querySelector("main.chat-app");
    const historySlide = document.querySelector(
      "#history-slide",
    ) as HTMLElement;
    const chatDetailToggleEl = document.querySelector("#chat-detail-toggle");

    new Swiper(historySlide!, {
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
