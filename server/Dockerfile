FROM debian:bookworm-slim

ARG TARGETARCH

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

COPY binaries/${TARGETARCH}/bvc-server /usr/local/bin/bvc-server
RUN chmod +x /usr/local/bin/bvc-server

RUN useradd -r -s /bin/false bvc && \
    mkdir -p /data && \
    chown bvc:bvc /data

USER bvc

EXPOSE 443
EXPOSE 8443/udp

VOLUME ["/data"]

ENTRYPOINT ["bvc-server"]
