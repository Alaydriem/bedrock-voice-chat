# The relay protocol is tied to the iroh release. This MUST match the `iroh`
# version in relay/lib/Cargo.toml: a mismatch fails when a peer tries to relay,
# not when this image builds.
ARG IROH_VERSION=1.0.3

FROM rust:1-bookworm AS build
ARG IROH_VERSION
RUN cargo install iroh-relay \
      --version ${IROH_VERSION} \
      --features server \
      --locked \
      --root /out

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=build /out/bin/iroh-relay /usr/local/bin/iroh-relay

# Relay HTTP and HTTPS, and QUIC address discovery.
EXPOSE 80/tcp 443/tcp 443/udp

ENTRYPOINT ["/usr/local/bin/iroh-relay"]
CMD ["--config-path", "/etc/iroh-relay/relay.toml"]
