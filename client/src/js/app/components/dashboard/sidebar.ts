// @ts-ignore
import murmurHash3 from "murmurhash3js";
import { mount } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { error } from '@tauri-apps/plugin-log';
import ServerAvatarIcon from "../../../../components/dashboard/ServerAvatarIcon.svelte";

export default class Sidebar {
  private serverList: Array<{ server: string, player: string }>;
  private currentServer: string;

  constructor(
    serverList: Array<{ server: string, player: string }>,
    currentServer: string
  ) {
    this.serverList = serverList;
    this.currentServer = currentServer;
  }

  async render() {
    const container = document.getElementById('dashboard-server-links');
      if (!container) {
        error("Container for server avatars not found");
        return;
      }

    container.innerHTML = '';

    const multiServerEnabled = await invoke<boolean>("get_feature_flag", { flag: "multi-server" })
        .catch(() => false);

    if (multiServerEnabled) {
      const addBtn = document.createElement('a');
      addBtn.href = '/login?addserver=true';
      addBtn.className = 'flex size-11 items-center justify-center rounded-lg text-2xl font-light ' +
          'text-slate-400 outline-hidden transition-colors duration-200 ' +
          'hover:bg-primary/20 dark:hover:bg-navy-300/20';
      addBtn.textContent = '+';
      addBtn.setAttribute('aria-label', 'Add Server');
      container.appendChild(addBtn);
    }

    this.serverList.forEach(({ server }) => {
      const bytes = new TextEncoder().encode(server);
      const byteString = Array.from(bytes)
        .map((byte) => String.fromCharCode(byte))
        .join('');
      const hash = murmurHash3.x86.hash128(byteString);

      mount(ServerAvatarIcon, {
        target: container,
        props: {
          id: hash,
          server: server,
          active: server === this.currentServer,
        },
      });
    });
  }
}