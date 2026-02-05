FROM alpine:latest

ARG TARGETARCH

COPY binaries/${TARGETARCH}/bvc-server /usr/local/bin/bvc-server

RUN chmod +x /usr/local/bin/bvc-server && \
    addgroup -S bvc && \
    adduser -S -G bvc -H -s /sbin/nologin bvc && \
    mkdir -p /data && \
    chown bvc:bvc /data

USER bvc

EXPOSE 443
EXPOSE 8443/udp

VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/bvc-server", "server"]
