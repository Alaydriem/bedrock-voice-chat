# BVC relay

An [iroh relay](https://github.com/n0-computer/iroh) BVC peers connect through
when they have no direct path to each other. It is the upstream `iroh-relay`
server, pinned and packaged — there is no BVC code here.

## What it does and does not do

It forwards opaque, end-to-end encrypted frames between endpoints. It holds no
BVC key material, terminates no BVC session, and cannot read voice or positions.
Peers that can reach each other directly stop using it after the handshake.

Two BVC deployments never need it at all: a bridge on the same host as the
server, and two hosts on one local network. Both reach each other at an address
the peer link already carries.

## Deploy

1. Point a DNS record at the host.
2. Set `hostname` in `relay.toml` to that name. The certificate is issued for it,
   so it must match what peers dial.
3. `IROH_RELAY_ACCESS_TOKEN=<token> docker compose up -d --build`

Ports 80/tcp, 443/tcp and 443/udp must be reachable. 80 is Let's Encrypt's
HTTP-01 challenge; 443/udp is QUIC address discovery, without which every peer
pair relays forever instead of finding a direct path.

## Wiring it to a BVC server

In each server's `config.hcl`:

```hcl
server {
  peer_relay_url = "https://relay.example.com"
}
```

Leave it unset and a peer link is reachable only at an address the far side
already holds — which is the same-host and local-network case, and needs no
relay at all.

## The access token

The relay admits only clients presenting a bearer token. BVC bakes its token in
at build time from `BVC_RELAY_ACCESS_TOKEN`; the relay reads the same value from
`IROH_RELAY_ACCESS_TOKEN` at runtime. The two must match.

**This is not a secret.** It ships inside every BVC binary and anyone who looks
can read it. It raises the cost of using this relay from "point at the URL" to
"extract a string from a binary", and nothing more is claimed for it. What
protects a call is the peer's key and its grant — the relay forwards opaque
encrypted frames either way, so a stranger who extracts the token gains free
bandwidth, not access to anyone's voice.

If that trade stops being acceptable, upstream also supports an allowlist of
endpoint ids:

```toml
[access]
allowlist = ["<endpoint-id>", "<endpoint-id>"]
```

That is genuinely restrictive, at the cost of a second copy of every peer
identity to keep in step with each server's `config.hcl` — and a failure mode
where peering works on a local network and silently not through the relay.

An operator running their own relay picks their own token and sets
`BVC_RELAY_ACCESS_TOKEN` in each server's environment, which overrides the
built-in value.

## Version pinning

`IROH_VERSION` must match the `iroh` version in `relay/lib/Cargo.toml`. The relay
protocol is tied to the iroh release, and a mismatch fails when a peer tries to
relay rather than when this image builds. `build-relay-docker.yml` reads the
version out of the lockfile rather than trusting the default in the Dockerfile.
