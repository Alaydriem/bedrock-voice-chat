# The BVC registry.
#
# An iroh endpoint that assigns hostnames and answers address observations. It relays
# nothing, so there is no HTTP server, no TLS and no certificate here: iroh's own
# handshake authenticates this host by node id.

FROM rust:1-bookworm AS build

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      pkg-config \
      libssl-dev \
      cmake \
      clang \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .

RUN cargo build --release --locked -p bvc-relay-service

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=build /src/target/release/bvc-relay-service /usr/local/bin/bvc-relay-service

# The registry endpoint and the operator-facing HTTPS surface.
EXPOSE 443/tcp 28286/udp

ENTRYPOINT ["/usr/local/bin/bvc-relay-service"]
CMD ["/etc/bvc-relay/config.hcl"]
