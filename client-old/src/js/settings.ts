import { invoke } from "@tauri-apps/api/tauri";
import { displayCachedImage } from "./utils/imgCache";

import Swiper from "swiper/bundle";
import Drawer from "./components/drawer";
import Popper from "./components/popper";
import Tab from "./components/tab";

import { StreamType } from "./bindings/StreamType";
import { AudioDevice } from "./bindings/AudioDevice";
import { AudioDeviceType } from "./bindings/AudioDeviceType";
import { Channel } from "./bindings/Channel";

export default class Settings {
  constructor() {
    const page = document.querySelector("#settings-page");
    if (page == null) {
      return;
    }
    const input_select = <HTMLInputElement>(
      document.querySelector("select#input")
    );
    const output_select = <HTMLInputElement>(
      document.querySelector("select#output")
    );
    invoke("get_devices")
      .then((devices) => devices as Array<AudioDevice>)
      .then((devices) => {
        for (const host in devices) {
          let io = devices[host];
          for (const device in io) {
            let d = devices[host][device];

            let option = document.createElement("option");
            option.value = d.name;
            option.text = d.name;
            if (d.io == "InputDevice") {
              input_select?.append(option);
            } else {
              output_select?.append(option);
            }
          }
        }
      });

    input_select?.addEventListener("change", () => {
      invoke("input_stream", { device: input_select?.value });
    });
    output_select?.addEventListener("change", () => {
      invoke("output_stream", { device: output_select?.value });
    });
  }
}
