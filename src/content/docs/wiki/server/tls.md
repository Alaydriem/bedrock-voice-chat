---
title: TLS certificates
description: An assigned hostname, automatic certificates over ACME DNS-01, or bring your own.
sidebar:
  label: TLS certificates
  order: 3
---

Bedrock Voice Chat requires a valid TLS certificate. Client apps validate it the way a browser does.

:::danger
Self-signed certificates **will not work**. The app refuses the connection and there is no override.
:::

Three options. They are mutually exclusive, and the server refuses to start with more than one configured.

| | Use when |
|---|---|
| **An assigned hostname** | You have no domain. BVC gives you a name and keeps its certificate renewed. Work in progress, and limited to select Patreon and YouTube members. |
| **ACME** | You have a domain and a supported DNS provider. BVC issues and renews for you. |
| **Your own PEM files** | You already have a certificate, or your DNS provider is not supported. |

The server appends the name it is given to its own SANs and configures its own certificate. See [assigned hostnames](/wiki/server/enrollment/).

The rest of this page is for the two options where you bring the name.

## ACME

BVC proves domain control with a DNS record, not an HTTP request. **Port 80 never has to be open**, and the host does not need to answer HTTP at all. Renewal is automatic.

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

## Bring your own certificate

```hcl
server {
    tls {
        names       = ["example.bedrockvc.stream"]
        certificate = "/etc/bvc/fullchain.pem"
        key         = "/etc/bvc/privkey.pem"
    }
}
```

:::caution[Fullchain, not the leaf]
A leaf-only certificate validates on a machine that already has the intermediate cached and fails on a fresh phone. It works when you test it and breaks for your users, which is a miserable bug to chase.
:::

With BYOC you are responsible for rotation.