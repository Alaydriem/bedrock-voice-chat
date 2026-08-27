// The BVC registry, configured for a local proving run.
//
// Copy this to config.local.hcl, which is gitignored. The differences from
// config.example.hcl are all about running on one machine: a SQLite file under the
// worktree, and a page origin pointing at the Astro dev server rather than the
// published site.
//
// Everything secret still arrives from the environment. A referenced but unset
// variable is a hard failure at startup, never a silent empty string.

// A file, not `sqlite::memory:`. This holds the node key, the registry's own
// certificate and every assigned name, so an in-memory database would hand out a new
// identity on every run and leave the previous names published in the zone with nothing
// answering for them.
database_url = "sqlite://.local/registry.db?mode=rwc"

zone = "bedrockvc.stream"
enroll_port = 28286

// Console on stderr is unconditional; the rotating JSON file lands here. `RUST_LOG`
// overrides the level wholesale — `RUST_LOG=debug,iroh=info` when chasing a dial.
logger {
  level = "info"
  path  = ".local/logs"
}

// Deliberately low for a local run. The ceiling exists so a loop cannot spend the
// certificate authority's weekly allowance; on one machine there is no reason for it
// to be near the production number.
weekly_certificate_ceiling = 5

discord {
  guild_id      = "${env.BVC_RELAY_DISCORD_GUILD_ID}"
  bot_token     = "${env.BVC_RELAY_DISCORD_BOT_TOKEN}"
  client_id     = "${env.BVC_RELAY_DISCORD_CLIENT_ID}"
  client_secret = "${env.BVC_RELAY_DISCORD_CLIENT_SECRET}"

  // Every role that qualifies. An empty list refuses everyone, so a run that always
  // answers `not_entitled` usually means this was left unset rather than that the
  // check is wrong.
  qualifying_role_ids = ["${env.BVC_RELAY_DISCORD_ROLE_ID}"]
}

cloudflare {
  api_token = "${env.BVC_RELAY_CLOUDFLARE_TOKEN}"
  zone_id   = "${env.BVC_RELAY_CLOUDFLARE_ZONE_ID}"
}

http {
  // Reached through a hosts entry pointing this name at 127.0.0.1. Discord redirects a
  // browser here, so the name has to resolve on the machine running the browser.
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

  // The Astro dev server, not the published site. This is compared against the
  // browser's Origin header when the page redeems a claim, and a mismatch fails as a
  // CORS error in the console rather than as anything naming this field.
  //
  // `astro dev --host` prints the port it took. It is 4321 unless something else has
  // it.
  page_origin = "http://localhost:4321"

  acme "cloudflare" {
    email     = "${env.BVC_REGISTRY_ACME_EMAIL}"
    api_token = "${env.BVC_REGISTRY_CLOUDFLARE_TOKEN}"

    // Staging. It exercises account creation, the order, DNS-01 through real
    // Cloudflare, real propagation and the write into the database, and spends nothing
    // from the production allowance.
    //
    // The certificate it returns is untrusted, so a browser warns once and has to be
    // told to continue. Comment this line out for the production run, and delete the
    // stored row first — a valid staging certificate in the database means no reorder
    // happens.
    directory = "https://acme-staging-v02.api.letsencrypt.org/directory"
  }
}
