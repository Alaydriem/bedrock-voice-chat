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
  switch (cmd[0]) {
    case "!version":
      event.cancel = true;
      system.run(() => {
        v.version();
      });
      break;
    default:
      break;
  }
});

const bvc_server = mcsa.variables.get("bvc_server");
console.log(bvc_server);

system.runInterval(async (e) => {
  let data = [];
  let players = world.getAllPlayers();
  players.forEach((player) => {
    let p = {
      name: player.name,
      dimension: player.dimension.id,
      coordinates: player.location,
      deafen: player.isInWater || player.isSneaking,
    };
    data.push(p);
  });

  let request = new HttpRequest(bvc_server + "/api/position");
  request.setBody(JSON.stringify(data));
  request.setMethod(HttpRequestMethod.Post);
  request.setHeaders([new HttpHeader("Content-Type", "application/json")]);
  request.setTimeout(1);

  await http
    .request(request)
    .then((response) => {})
    .catch((error) => {});
}, 5);
