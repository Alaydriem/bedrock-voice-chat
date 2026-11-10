// The BVC registry.
//
// An iroh endpoint that assigns hostnames to entitled members and tells any server
// the address it was seen at. It relays NOTHING: two peers that cannot reach each
// other directly do not connect, and no traffic is ever carried through this host.
//
// Secrets are read from the environment rather than written here. A referenced but
// unset variable is a hard failure at startup, never a silent empty string.

// The only durable state. The node key and the registry's own certificate live here
// too, so this database is the whole of what a deployment has to keep: there is no
// volume to mount and nothing on disk to back up alongside it.
database_url = "${env.BVC_RELAY_DATABASE_URL}"

// The apex assigned names sit directly beneath, e.g.
// creeper-diorite-badlands.bedrockvc.stream
zone = "bedrockvc.stream"

// Console on stderr is unconditional; the rotating JSON file is this directory. An
// unwritable path degrades to console-only and says so once rather than stopping the
// start — a log is what the process did, not state it depends on.
logger {
  level = "info"
  path  = "/var/log/bvc-relay"
}

// The UDP port the enrollment endpoint binds. Pinned rather than left to the
// operating system: every enrolled server stores the address this port is part of.
enroll_port = 28286

// The public address servers reach the enrollment endpoint at. Required when the
// registry runs on a bridged container network: the endpoint can see only loopback and
// the bridge, and the enrollment link it logs names nothing a server can dial. An IP
// takes `enroll_port`; `ip:port` names a published port that differs from it. Empty
// leaves the link as the endpoint sees itself, which is right under host networking.
advertise_address = "${env.BVC_RELAY_ADVERTISE_ADDRESS}"

// First issuances the certificate authority will accept per week for this zone.
// Renewals are exempt and are not counted against it. Raise this only after the
// authority has granted a matching increase.
weekly_certificate_ceiling = 50

discord {
  guild_id = "${env.BVC_RELAY_DISCORD_GUILD_ID}"

  // A bot resident in the guild. Read-only: it fetches a member's roles and nothing
  // else. Not the member's own OAuth token, which is short-lived and carries no
  // refresh, so a daily re-check against it would mean re-authenticating every
  // operator every day.
  bot_token = "${env.BVC_RELAY_DISCORD_BOT_TOKEN}"

  // The web UI's OAuth application, used once per operator at enrollment to learn
  // their Discord user id.
  client_id = "${env.BVC_RELAY_DISCORD_CLIENT_ID}"
  client_secret = "${env.BVC_RELAY_DISCORD_CLIENT_SECRET}"

  // Every role that qualifies for an assigned name. Patreon and YouTube both sync
  // into Discord roles, so either alone is enough. An empty list refuses everyone.
  qualifying_role_ids = ["REPLACE_WITH_ROLE_ID"]
}

cloudflare {
  // Scoped to Zone.DNS:Edit on this zone and nothing else. The highest-value secret
  // in the deployment: it can rewrite every operator's address and complete a
  // challenge for any name in the zone.
  api_token = "${env.BVC_RELAY_CLOUDFLARE_TOKEN}"
  zone_id = "${env.BVC_RELAY_CLOUDFLARE_ZONE_ID}"
}

// The operator-facing HTTP surface. TLS is always enforced; there is no setting here
// that disables it, because this is where enrollment tokens are handed out.
http {
  // The name this registry is reached by, the name its certificate is issued for, and
  // the name Discord's registered redirect URI is built from.
  hostname = "registry.bedrockvoicechat.com"

  // The address the HTTPS listener binds.
  //
  // `::` is DUAL-STACK here, not IPv6-only: the listener clears IPV6_V6ONLY itself, so
  // IPv4 clients are served on every platform rather than only on the ones whose
  // default allows it. Windows defaults that flag on, which is why a bare `[::]` bind
  // refuses IPv4 there in most software.
  //
  // Override with `0.0.0.0` for IPv4 alone, or a specific address to bind one
  // interface.
  bind = "::"

  // The one origin allowed to redeem a claim. Everything else here is browser
  // navigation and carries no CORS headers.
  page_origin = "https://bedrockvoicechat.com"

  // The registry's own certificate. The zone is discovered from the hostname, so this
  // token must have access to BOTH zones: bedrockvc.stream for assigned names and
  // bedrockvoicechat.com for this one. A token scoped to one fails at the other's
  // first issuance with a permissions error.
  acme "cloudflare" {
    email     = "${env.BVC_REGISTRY_ACME_EMAIL}"
    api_token = "${env.BVC_REGISTRY_CLOUDFLARE_TOKEN}"
  }
}

// Play names: <label>.play.bedrockvc.stream, answered by this registry's own DNS
// server and certified by a step-ca reachable only on loopback. Absent turns all of it
// off.
play {
  zone = "play.bedrockvc.stream"

  // New play names per observed source address per 24 hours, and across the registry
  // per hour. A name with no check-in for release_after_days is released.
  claims_per_address_per_day = 3
  claims_per_hour            = 200
  release_after_days         = 30

  dns {
    // Required, with no wildcard default: systemd-resolved holds 127.0.0.53:53, and a
    // wildcard bind collides with it. The registry refuses to start if an address here
    // is taken.
    bind = ["203.0.113.10:53"]

    // The public NS set. Equal to the NS records delegating the zone at its parent.
    nameservers = ["ns1.bedrockvc.stream.", "ns2.example.net."]

    // The SOA mname: this registry's own NS name, listed above or hidden.
    primary    = "ns1.bedrockvc.stream."
    hostmaster = "hostmaster.bedrockvc.stream."
    ttl        = 60

    // Secondaries hold a full copy, pulled by AXFR after a NOTIFY. Each one needs the
    // tsig block below.
    secondaries = [
      { transfer_from = ["198.51.100.20"], notify = "198.51.100.20:53" },
    ]

    tsig {
      name   = "play-transfer"
      secret = "${env.BVC_RELAY_PLAY_TSIG_SECRET}"
    }
  }

  // The URL's host is fixed and never looked up in public DNS: it connects to `connect`
  // instead. `connect` is `127.0.0.1:9000` when step-ca shares the host's network, or the
  // compose service name (`play-ca:9000`) on a Docker bridge. A name is looked up again
  // for every new connection. The URL's port has to equal the port in `connect`.
  ca {
    url             = "https://ca.play.bedrockvc.stream:9000"
    connect         = "127.0.0.1:9000"
    provisioner     = "registry"
    provisioner_kid = "${env.BVC_RELAY_PLAY_PROVISIONER_KID}"
    provisioner_key = "${env.BVC_RELAY_PLAY_PROVISIONER_KEY}"
    root            = "/var/lib/bvc-relay/play-root.pem"
  }
}
