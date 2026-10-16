---
title: TLS certificates
description: An assigned hostname, automatic certificates over ACME DNS-01, or bring your own.
sidebar:
  label: TLS certificates
  order: 3
---

Bedrock Voice Chat requires a valid TLS certificate. The client apps check it the same way a browser does.

:::danger
Self-signed certificates **will not work**. The app refuses the connection, and you cannot turn this check off.
:::

There are three options. Use only one. If you configure more than one, the server does not start.

| Option | When to use |
|---|---|
| **An assigned hostname** | You have no domain. BVC gives you a name and renews its certificate. In development, and available only to some Patreon and YouTube members. See [assigned hostnames](/wiki/server/enrollment/). |
| **ACME** | You have a domain and a supported DNS provider. BVC gets and renews the certificate. |
| **Your own PEM files** | You already have a certificate, or your DNS provider is not supported. |

## ACME

BVC uses a DNS record to prove that you control the domain (ACME DNS-01). You do not need to open port 80 or run an HTTP service. Renewal is automatic.

BVC supports two DNS providers.

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

:::caution[Use a scoped API token]
Create an API token on Cloudflare's **API Tokens** page. Limit it to one zone, with **DNS edit** permission. Do not use the global API key. It gives BVC control of your whole Cloudflare account.
:::

### acme-dns

Use acme-dns when your DNS provider has no supported API, or when you do not want to give BVC credentials for your main DNS zone.

Add a CNAME that points your `_acme-challenge` record to the acme-dns server. BVC writes only to the acme-dns server. It does not change your DNS zone.

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

Register with the acme-dns server first. Registration gives you the username, the password, the subdomain, and the CNAME to add.

## Your own certificate

```hcl
server {
    tls {
        names       = ["example.bedrockvc.stream"]
        certificate = "/etc/bvc/fullchain.pem"
        key         = "/etc/bvc/privkey.pem"
    }
}
```

:::caution[Use the full chain]
Use `fullchain.pem`. A certificate without the intermediate certificate works on a computer that already has the intermediate cached. It fails on a new phone.
:::

If you use your own certificate files, replace them before they expire.
