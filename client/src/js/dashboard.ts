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
export default class Dashboard {
  constructor() {
    const page = document.querySelector("#dashboard-page");
    if (page == null) {
      return;
    }

    // Load the players gamer picture
    const profilePicture = document.querySelector(
      "#profile-wrapper #profile-ref img.rounded-full",
    );
    const settingsFlyoverPicture = document.querySelector(
      "#profile-box .avatar img.rounded-full",
    );

    // Start the QUIC network stream if it isn't active
    invoke("is_network_stream_active").then((result) => {
      if (!result) {
        invoke("network_stream");
      }
    });

    invoke("is_audio_stream_active").then((result) => {
      if (!result) {
        // Start the audio streams if they aren't active
        // We'll use the default audio interface if there's none in the cache
        invoke("input_stream", { device: "default" });
        invoke("output_stream", { device: "default" });
      }
    });

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

    // Show the server list
    this.show_servers();
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

    // Create new group button
    const createNewGroupBtn = document.querySelector("#create-new-group-btn");
    createNewGroupBtn?.addEventListener(
      "click",
      this.create_new_group.bind(this),
    );

    const groupsBottomToolbarLink = <HTMLElement>(
      document.querySelector("#groups-bottom-toolbar-link")
    );
    groupsBottomToolbarLink?.addEventListener(
      "click",
      this.update_groups_sidebar.bind(this),
    );
    groupsBottomToolbarLink?.click();

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

  async update_groups_sidebar(e) {
    if (e !== null) {
      e.preventDefault();
    }

    let me = this;
    invoke("get_channels")
      .then(async (data) => data as Array<Channel>)
      .then((data) => {
        document.querySelectorAll("#group-id")?.forEach((chan) => {
          chan.remove();
        });

        data.forEach((d) => {
          me.add_channel(d);
        });

        let channels = document.querySelectorAll("#group-id");
        let allIds = [];
        channels.forEach((channel) => {
          allIds.push(channel.getAttribute("channel-id"));
        });

        channels.forEach((channel) => {
          let id = channel?.getAttribute("channel-id");
          channel.addEventListener("click", async (e) => {
            for (const idd of allIds) {
              if (idd != id) {
                this.leave_channel(idd);
              }
            }

            this.join_channel(id);
          });
        });
      })
      .catch((error) => {
        console.log(error);
      });
  }

  async create_new_group() {
    let group_name = <HTMLInputElement>(
      document.querySelector("#new-group-name-inpt")
    );

    let group_name_text = group_name?.value;
    if (group_name_text == "") {
      return;
    }

    let me = this;
    invoke("create_channel", { name: group_name_text })
      .then((data) => data as Channel)
      .then(async (data) => {
        await me.add_channel(data);
        await this.update_groups_sidebar(null);
      })
      .catch((error) => {
        console.log(error);
        console.log("failed to create group");
      });
  }

  async add_channel(data: Channel) {
    const groupTemplate = <HTMLElement>(
      document.querySelector("template#group-chat-template")
    );
    const groupTemplateContent = groupTemplate?.content.cloneNode(true);

    groupTemplateContent
      .querySelector("#group-id")
      .setAttribute("channel-id", data.id);

    groupTemplateContent.querySelector("#group-owner").innerHTML = data.creator;
    groupTemplateContent.querySelector("#group-name").innerHTML = data.name;
    groupTemplateContent.querySelector("#group-member-count").innerHTML =
      data.players.length;

    document.querySelector("#group-list")?.append(groupTemplateContent);
  }

  async join_channel(id: string) {
    invoke("join_channel", { id: id }).then(async () => {
      console.log("Join: " + id);
      let body = document.querySelector("#dashboard-content-section");
      body.innerHTML = "";

      let classes = "btn font-medium text-white".split(" ");
      let leaveBtn = document.createElement("button");
      classes.forEach((c) => {
        leaveBtn.classList.add(c);
      });
      leaveBtn.classList.add("bg-warning");

      leaveBtn.innerHTML = "Leave Group";
      leaveBtn.setAttribute("group-id", id);

      let destroyBtn = document.createElement("button");
      classes.forEach((c) => {
        destroyBtn.classList.add(c);
      });
      destroyBtn.classList.add("bg-error");
      destroyBtn.innerHTML = "Destroy Group";
      destroyBtn.setAttribute("group-id", id);

      leaveBtn.addEventListener("click", (e) => {
        let target = e.target as HTMLElement;
        let eventId = target.getAttribute("group-id");
        this.leave_channel(eventId);
      });

      destroyBtn.addEventListener("click", (e) => {
        let target = e.target as HTMLElement;
        let eventId = target.getAttribute("group-id");
        this.destroy_channel(eventId);
      });

      body?.appendChild(leaveBtn);
      body?.appendChild(destroyBtn);

      await this.update_groups_sidebar(null);
    });
  }

  leave_channel(id: string) {
    invoke("leave_channel", { id: id }).then(async () => {
      await this.update_groups_sidebar(null);
      let body = document.querySelector("#dashboard-content-section");
      body.innerHTML = "";
    });
  }

  destroy_channel(id: string) {
    invoke("delete_channel", { id: id }).then(async () => {
      await this.update_groups_sidebar(null);
      let body = document.querySelector("#dashboard-content-section");
      body.innerHTML = "";
    });
  }

  show_servers() {
    invoke("get_credential_raw", { key: "server_list" })
      .then((servers) => servers as string)
      .then((servers) => JSON.parse(servers))
      .then((servers) => {
        let serverList = document.querySelectorAll("#server-list-section a");
        serverList.forEach((el) => {
          el.remove();
        });
        const serverTemplate = <HTMLElement>(
          document.querySelector("template#server-list")
        );
        const serverListSection = document.querySelector(
          "#server-list-section",
        );
        for (const server of servers) {
          const serverTemplateContent =
            serverTemplate?.content?.cloneNode(true);

          let href = serverTemplateContent.querySelector("a");
          href.setAttribute("data-server-conn-string", server);

          serverListSection?.append(serverTemplateContent);
        }
      });
  }
}
