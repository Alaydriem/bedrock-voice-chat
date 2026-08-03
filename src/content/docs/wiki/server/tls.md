---
title: TLS certificates
description: Automatic certificates over ACME DNS-01, or bring your own.
sidebar:
  label: TLS certificates
  order: 3
---

Client apps validate your certificate the way a browser does. Self-signed will not work.

You have two options, and they are mutually exclusive — the server refuses to start with both configured.

| | Use when |
|---|---|
| **ACME** | You have a domain and a supported DNS provider. BVC issues and renews for you. |
| **Your own PEM files** | You already have a certificate, or your DNS provider is not supported. |

## ACME

BVC proves domain control through a DNS record rather than an HTTP request, so **port 80 never has to be open** and the host does not need to answer HTTP at all. Renewal happens without you.

Two providers are supported.

### Cloudflare

```hcl
server {
    tls {
        names = ["example.bedrockvc.stream"]

        acme {
            email     = "you@example.com"
            provider  = "cloudflare"
            api_token = "${env.CF_API_TOKEN}"
        }
    }
}
```

:::caution[Use a scoped token, not your global key]
Create an API token under Cloudflare's **API Tokens** page, scoped to the single zone, with **DNS edit** permission. A global API key would give BVC control of your entire Cloudflare account.
:::

### acme-dns

Use this when your DNS provider has no supported API, or when you would rather not hand BVC credentials to your real zone at all.

You delegate one `_acme-challenge` record to an acme-dns server with a CNAME. BVC only ever writes there, never to your zone.

```hcl
server {
    tls {
        names = ["example.bedrockvc.stream"]

        acme {
            email      = "you@example.com"
            provider   = "acme-dns"
            server_url = "https://auth.acme-dns.io"
            username   = "${env.ACME_DNS_USERNAME}"
            password   = "${env.ACME_DNS_PASSWORD}"
            subdomain  = "${env.ACME_DNS_SUBDOMAIN}"
        }
    }
}
```

Register with the acme-dns server first. It returns the username, password, subdomain, and the CNAME to add.

### Which names end up on the certificate

The DNS entries in `tls.names`. IP entries are skipped — an ACME certificate cannot carry one.

Override with `domains` when the certificate should differ from what clients dial:

```hcl
acme {
    domains = ["example.bedrockvc.stream", "voice.bedrockvc.stream"]
}
```

With no DNS entries in `tls.names` and no `domains`, startup fails rather than requesting an empty certificate.

### Test against staging first

Let's Encrypt rate-limits issuance per domain per week, and a misconfigured retry loop will burn through it.

```hcl
acme {
    directory = "https://acme-staging-v02.api.letsencrypt.org/directory"
}
```

Staging certificates are signed by an untrusted root, so client apps will reject them. That is expected — you are testing whether issuance succeeds, not whether clients connect. Remove the line once it does.

## Your own certificate

```hcl
server {
    tls {
        names       = ["example.bedrockvc.stream"]
        ips         = ["203.0.113.1"]
        certificate = "/etc/bvc/fullchain.pem"
        key         = "/etc/bvc/privkey.pem"
    }
}
```

:::caution[Fullchain, not the leaf]
A leaf-only certificate validates on a machine that already has the intermediate cached and fails on a fresh phone. It works when you test it and breaks for your users, which is a miserable bug to chase.
:::

Renewal is yours. Restart the server after replacing the files.

## `names` and `ips`

These are the SANs. Every address anyone actually dials has to appear in one of them, or the handshake fails on that address specifically — while working fine on the others.

Include the public hostname. Add `127.0.0.1` and your LAN IP if you test locally.

## There is a second certificate, and it is not yours

| | Purpose | Managed by |
|---|---|---|
| HTTPS and QUIC TLS | Clients trusting your server | You, or ACME |
| mTLS CA | Your server authenticating clients | BVC |

BVC generates its own certificate authority in `certs_path` (default `./certificates`) and issues every player a client certificate from it. That is what makes a signed-in client provably itself on later connections.

:::danger[Back up `certs_path`]
Delete or replace `ca.crt` and every client certificate ever issued becomes invalid at once. Every player is signed out and has to redeem a new login code.

Nothing fails at startup when this happens. It presents as a mass authentication outage with no obvious cause.
:::

Back it up alongside your database, not separately — they have to be restored together.

## Checking it

```bash
curl -v https://example.bedrockvc.stream/api/config
```

A clean handshake and JSON back means TLS is right. A certificate error here is the same error every client app will hit.
