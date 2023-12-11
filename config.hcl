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
        certs_path = "../certificates"
    }

    minecraft {
        access_token = "kB49Q%CwPjY8Z2@Aza2sjkH9PdVd66C9"
        client_id = "a17f9693-f01f-4d1d-ad12-1f179478375d"
        client_secret = "tuB8Q~BcDa2FejrcPJLAQrJWDeTW5Uilq8BsaduS"
    }
}

log {
    level = "info"
    out = "stdout"
}

redis {
    host = "10.57.2.4"
    port = 16379
    database = ""
}