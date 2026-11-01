// The BVC registry, configured to prove play names on one machine.
//
// docs/runbooks/play-zone-proof.md walks through it. Copy this to .local/config.play.hcl,
// which is gitignored, and run the registry from relay/service so the relative paths
// below resolve.
//
// This file carries no Discord, Cloudflare or Let's Encrypt secret. The play plane does
// not use them. The registry still reads those blocks and still needs a certificate for
// its own HTTPS name, so the values below are placeholders and the runbook seeds a
// self-signed certificate into the database instead of ordering one.
//
// Two values arrive from the environment, because the key is a PEM and the key id is
// printed by `mise run play-ca-init`:
//
//   BVC_PLAY_PROVISIONER_KID  `kid` in .play-ca/secrets/provisioner_public.jwk
//   BVC_PLAY_PROVISIONER_KEY  the contents of .play-ca/secrets/provisioner_key.pem
//
// A referenced but unset variable is a hard failure at startup.

// A file, not `sqlite::memory:`. It holds the node key, so the peerlink the server is
// built with stays valid across registry restarts, and the play names that make up the
// zone.
database_url = "sqlite://.local/play-proof.db?mode=rwc"

zone = "bedrockvc.stream"
enroll_port = 28286

logger {
  level = "info"
  path  = ".local/logs"
}

// Placeholders. Nothing in a play-only run contacts Discord or Cloudflare.
discord {
  guild_id            = "0"
  bot_token           = "unused"
  client_id           = "0"
  client_secret       = "unused"
  qualifying_role_ids = ["0"]
}

cloudflare {
  api_token = "unused"
  zone_id   = "unused"
}

http {
  hostname    = "registry.bedrockvoicechat.com"
  page_origin = "http://localhost:4321"

  // Loopback and an unprivileged port: nothing here needs to be reached from outside.
  bind = "127.0.0.1"
  port = 8443

  // Required by the registry, never used while the seeded certificate is valid. Without a
  // stored certificate for `hostname` the registry orders one and fails against the
  // placeholder token.
  acme "cloudflare" {
    email     = "proof@example.invalid"
    api_token = "unused"
  }
}

play {
  // A check-in may name a loopback address, which the server in this run does.
  allow_private_probe_targets = true

  // An unprivileged loopback port. Not 5353: that is multicast DNS, which Windows and
  // macOS hold open, and the bind fails with "forbidden by its access permissions".
  // Query it with:
  //   nslookup -port=5300 -type=A <name>.play.bedrockvc.stream 127.0.0.1
  dns {
    bind        = ["127.0.0.1:5300"]
    nameservers = ["ns1.bedrockvc.stream."]
    primary     = "ns1.bedrockvc.stream."
    hostmaster  = "hostmaster.bedrockvc.stream."
  }

  // The local step-ca from `mise run play-ca`. The URL's host is fixed and resolved to
  // `connect`; the URL's port has to equal the port in `connect`.
  ca {
    url             = "https://ca.play.bedrockvc.stream:9000"
    connect         = "127.0.0.1:9000"
    provisioner     = "registry"
    provisioner_kid = "${env.BVC_PLAY_PROVISIONER_KID}"
    provisioner_key = "${env.BVC_PLAY_PROVISIONER_KEY}"
    root            = ".play-ca/certs/root_ca.crt"
  }
}
