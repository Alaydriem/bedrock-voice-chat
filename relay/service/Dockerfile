# The BVC registry.
#
# An iroh endpoint that assigns hostnames to entitled members, answers address
# observations, and hands out enrollment tokens over HTTPS. It relays nothing: two
# peers that cannot reach each other directly do not connect.
#
# The binary is built on the runner and copied in, the same way the server image is
# assembled. Compiling here instead would need credentials for the private git
# dependency `common` carries — cargo loads that source while resolving the workspace
# even though this crate never compiles it — and a build container cannot see the ones
# the workflow sets up. It would also put arm64 under QEMU.

FROM debian:bookworm-slim

ARG TARGETARCH

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY binaries/${TARGETARCH}/bvc-relay-service /usr/local/bin/bvc-relay-service

# The registry endpoint and the operator-facing HTTPS surface.
EXPOSE 443/tcp 28286/udp

ENTRYPOINT ["/usr/local/bin/bvc-relay-service"]
CMD ["/etc/bvc-relay/config.hcl"]
