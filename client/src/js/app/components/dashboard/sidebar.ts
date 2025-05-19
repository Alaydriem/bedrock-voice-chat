// @ts-ignore
import murmurHash3 from "murmurHash3js";
import { mount } from 'svelte';
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
        console.error("Container for server avatars not found");
        return;
      }

    container.innerHTML = '';

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