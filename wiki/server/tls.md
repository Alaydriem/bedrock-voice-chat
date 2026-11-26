---
title: "TLS certificates"
description: "A play name, an assigned hostname, automatic certificates over ACME DNS-01, or bring your own."
source: https://www.bedrockvoicechat.com/wiki/server/tls/
---
Bedrock Voice Chat requires a valid TLS certificate. The client apps check it the same way a browser does.

:::danger
Self-signed certificates **will not work**. The app refuses the connection, and you cannot turn this check off.
:::

There are four options. Use only one. If you configure more than one, the server does not start.

| Option | When to use |
|---|---|
| **A play name** | You have no domain. BVC gives the server a `*.play.bedrockvc.stream` name and renews its certificate. Nothing to set up. See [play name](#play-name). |
| **An assigned hostname** | You have no domain. BVC gives you a name and renews its certificate. In development, and available only to some Patreon and YouTube members. See [assigned hostnames](/wiki/server/enrollment/). |
| **ACME** | You have a domain and a supported DNS provider. BVC gets and renews the certificate. |
| **Your own PEM files** | You already have a certificate, or your DNS provider is not supported. |

## Play name

Bedrock Voice Chat play names give a server a name and a TLS certificate with no domain. The BVC registry assigns a name in the form `adjective-noun-place-NNNN.play.bedrockvc.stream` and a certificate for that name. The certificate is valid for 15 days. The server renews it automatically, without a restart. You do not need a domain, a DNS provider, or an open port 80.

```hcl
server {
    tls {
        play {}
    }
}
```

The empty block is a complete configuration.

:::caution
In a YAML config, write `play: {}` with the braces. `play:` with nothing after it is empty, and play mode stays off.
:::

To find the server's name, look in the server log for `Play name assigned`. The server keeps the same name after a restart. Players sign in with `https://<name>:<port>`, for example `https://quiet-otter-harbor-0420.play.bedrockvc.stream:28280`. If `server.port` is `443`, leave out the port.

### Address

The registry publishes the server's IP address as the name's DNS record. By default, it uses the address that it sees the server connect from. If players reach the server at a different address, set it:

```hcl
server {
    tls {
        play {
            address = "203.0.113.10"
        }
    }
}
```

- Use an IP address. Do not use a hostname. If the value is not an IP address, the server does not start.
- Use a public address. The registry refuses a private address, such as `192.168.x.x` or `10.x.x.x`.

### Registry check-in

- The server checks in with the registry every hour.
- If the registry is not available, the server continues to run with the certificate that it has.
- If the registry releases the name, the server claims a new name. The log says so. Give your players the new name.

### Limits

- Use only one certificate source. Do not use `play` with `certificate` and `key`, with `acme`, or with `server.enrollment.token`.
- A Java mod in external mode cannot connect to a server that uses a play name. Use embedded mode, or give the external server a domain.
- You cannot change the name. To use a name that you choose, use [ACME](#acme) with your own domain.

### Environment variables

`BVC_PLAY=true` turns on play mode. `BVC_PLAY_ADDRESS` sets the address. See [environment variables](/wiki/reference/environment-variables/#play-name).

### Bedrock Addon on the same host

When the Bedrock Addon runs on the same host as the server, set `server.mod_loopback = true`. The Addon then connects to `http://127.0.0.1:<port>` and needs no certificate. See [`mod_loopback`](/wiki/reference/configuration/#server).

```hcl
server {
    mod_loopback = true

    tls {
        play {}
    }
}
```

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