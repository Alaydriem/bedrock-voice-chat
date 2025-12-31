import { world, system } from '@minecraft/server';
import {
  HttpHeader,
  HttpRequest,
  HttpRequestMethod,
  http,
} from '@minecraft/server-net';
import { variables } from '@minecraft/server-admin';
import { Payload } from './dto';

const bvc_server: string = variables.get('bvc_server');
const access_token: string = variables.get('bvc_access_token');
const debug: boolean = variables.get("bvc_debug");

const POLL_INTERVAL = 5;
const MIN_PLAYERS = 2;
const REQUEST_TIMEOUT = 1;

console.info("[BVC] Connecting to: " + bvc_server);
system.runInterval(async () => {
  const players = world.getAllPlayers();

  if (!debug) {
    if (players.length < MIN_PLAYERS) {
      return;
    }
  }

  try {
    const payload = Payload.fromPlayers(players);

    const request = new HttpRequest(`${bvc_server}/api/position`);
    request.setBody(payload.toJSONString());
    request.setMethod(HttpRequestMethod.Post);
    request.setHeaders([
      new HttpHeader('Content-Type', 'application/json'),
      new HttpHeader('X-MC-Access-Token', access_token),
      new HttpHeader('Accept', 'application/json'),
    ]);
    request.setTimeout(REQUEST_TIMEOUT);

    await http
      .request(request)
      .then(() => {})
      .catch((error) => {
        console.warn('Failed to send player data:', error);
      });
  } catch (error) {
    console.error('Error creating player payload:', error);
  }
}, POLL_INTERVAL);
