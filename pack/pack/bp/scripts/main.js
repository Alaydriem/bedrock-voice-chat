"use strict";

import { world, system } from "@minecraft/server";
import {
  HttpClient,
  HttpHeader,
  HttpRequest,
  HttpRequestMethod,
  http,
} from "@minecraft/server-net";

 import { variables } from "@minecraft/server-admin";

const bvc_server = variables.get("bvc_server");
const access_token = variables.get("bvc_access_token");

system.runInterval(async (e) => {
  let data = [];
  let players = world.getAllPlayers();
  players.forEach((player) => {
    let p = {
      name: player.name,
      dimension: player.dimension.id.replace("minecraft:", ""),
      coordinates: player.location,
      deafen: player.isSneaking,
    };
    data.push(p);
  });

  let request = new HttpRequest(bvc_server + "/api/mc");
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
