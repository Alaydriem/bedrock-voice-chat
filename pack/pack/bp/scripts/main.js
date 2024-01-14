"use strict";

import { world, system } from "@minecraft/server";
import { Version } from "./version";
import {
  HttpClient,
  HttpHeader,
  HttpRequest,
  HttpRequestMethod,
  http,
} from "@minecraft/server-net";

import * as mcsa from "@minecraft/server-admin";

world.beforeEvents.chatSend.subscribe(async (event) => {
  let v = new Version(event, event.sender);
  let cmd = event.message.split(" ");
  if (cmd[0] == "!bvc") {
    switch (cmd[1] ?? "version") {
      default:
      case "version":
        event.cancel = true;
        system.run(() => {
          v.version();
        });
        break;
    }
  }
});

const bvc_server = mcsa.variables.get("bvc_server");
const access_token = mcsa.variables.get("bvc_access_token");

system.runInterval(async (e) => {
  let data = [];
  let players = world.getAllPlayers();
  players.forEach((player) => {
    let p = {
      name: player.name,
      dimension: player.dimension.id.replace("minecraft:", ""),
      coordinates: player.location,
      deafen: player.isInWater || player.isSneaking,
    };
    data.push(p);
  });

  let request = new HttpRequest(bvc_server + "/api/position");
  request.setBody(JSON.stringify(data));
  request.setMethod(HttpRequestMethod.Post);
  request.setHeaders([
    new HttpHeader("Content-Type", "application/json"),
    new HttpHeader("X-MC-Access-Token", access_token),
    new HttpHeader("Accept", "application/json"),
  ]);
  request.setTimeout(1);

  await http
    .request(request)
    .then((response) => {})
    .catch((error) => {});
}, 5);
