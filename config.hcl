server {
    listen = "0.0.0.0"
    port = 3000
    // This is the public address you want Homemaker to be available on. This should always be set
    public_addr = "127.0.0.1:3000"

    tls {
        so_resue_port = true
        certificate = "../server.crt"
        // Key must be in pkcs8 format
        key = "../server.key"
    }
}

log {
    level = "info"
    out = "stdout"
}

redis {
    host = "127.0.0.1"
    port = 6379
    database = ""
}