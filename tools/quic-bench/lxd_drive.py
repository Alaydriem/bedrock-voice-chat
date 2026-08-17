#!/usr/bin/env python3
"""Drive a BVC voice-load benchmark inside a throwaway LXD container.

Creates the container over the LXD HTTPS API, pushes the prebuilt binaries and a
benchmark script into it, runs the script, streams the output back, then deletes
the container. Clients and server are co-resident, so all traffic is loopback.

Configuration is entirely by environment; see tools/quic-bench/README.md.
"""
import json
import os
import ssl
import sys
import time
import urllib.request

HOST = os.environ.get("LXD_HOST", "https://10.57.2.3:8443")
CERTS = os.environ.get("LXD_CERTS", os.path.expanduser("~/.config/bvc-quic-bench"))
CA = os.environ.get("BVC_CA", "server/server/certificates")
NAME = os.environ.get("CT_NAME", "bvcbench")
CORES = os.environ.get("CT_CORES", "8")
MEM = os.environ.get("CT_MEM", "6GB")
IMAGE = os.environ.get("CT_IMAGE", "24.04")
SCRIPT = os.environ.get("BENCH_SCRIPT", os.path.join(os.path.dirname(__file__), "bench-in-container.sh"))

TARGET = os.environ.get("BVC_TARGET_DIR", os.path.expanduser("~/bvc-target"))
MER_TARGET = os.environ.get("MERIDIAN_TARGET_DIR", os.path.expanduser("~/mer-target"))

SERVER_BIN = os.environ.get(
    "BVC_SERVER_BIN", f"{TARGET}/x86_64-unknown-linux-musl/release/bvc-server"
)
BROADCAST_BIN = os.environ.get("BVC_BROADCAST_BIN", f"{TARGET}/release/examples/broadcast")
MERIDIAN_BIN = os.environ.get("MERIDIAN_BIN", f"{MER_TARGET}/release/meridian")

# Every variable the in-container scripts read. Absent ones fall back to the
# script's own defaults, so adding a knob here is the only wiring a new script
# needs.
PASSTHROUGH = {
    "MODE": "cells",
    "SHAPES": "5 1;5 5;10 10",
    "MERS": "0",
    "WINDOW": "60",
    "P": None,
    "S": None,
    "TRACE_SECS": None,
    "FRAME_MS": None,
    "BATCH_WAIT_MICROS": None,
}

ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE
ctx.load_cert_chain(f"{CERTS}/lxd-client.crt", f"{CERTS}/lxd-client.key")


def log(*a):
    print(*a, flush=True)


def req(method, path, body=None, raw=None, headers=None, timeout=300):
    url = HOST + path
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    r = urllib.request.Request(url, data=data, method=method)
    if body is not None and raw is None:
        r.add_header("Content-Type", "application/json")
    for k, v in (headers or {}).items():
        r.add_header(k, v)
    with urllib.request.urlopen(r, context=ctx, timeout=timeout) as resp:
        payload = resp.read()
    try:
        return json.loads(payload)
    except Exception:
        return payload


def wait(resp, timeout=900):
    """Block on an async LXD operation and return its metadata."""
    op = resp.get("operation") if isinstance(resp, dict) else None
    if not op:
        return resp
    out = req("GET", f"{op}/wait?timeout={timeout}", timeout=timeout + 30)
    md = out.get("metadata", {})
    if md.get("status") == "Failure":
        raise RuntimeError(f"operation failed: {md.get('err')}")
    return md


def destroy():
    for method, path, body in (
        ("PUT", f"/1.0/instances/{NAME}/state", {"action": "stop", "timeout": 60, "force": True}),
        ("DELETE", f"/1.0/instances/{NAME}", None),
    ):
        try:
            wait(req(method, path, body))
        except Exception:
            pass


def push(local, remote, mode=0o755, strip_cr=False):
    with open(local, "rb") as f:
        data = f.read()
    # Shell scripts are checked out with CRLF on Windows, and bash in the
    # container treats the carriage return as part of the command.
    if strip_cr:
        data = data.replace(b"\r\n", b"\n")
    req(
        "POST",
        f"/1.0/instances/{NAME}/files?path={remote}",
        raw=data,
        headers={
            "X-LXD-type": "file",
            "X-LXD-mode": oct(mode)[2:],
            "X-LXD-uid": "0",
            "X-LXD-gid": "0",
            "Content-Type": "application/octet-stream",
        },
        timeout=600,
    )
    log(f"  pushed {remote} ({len(data) / 1e6:.1f} MB)")


def environment():
    env = {
        "HOME": "/root",
        "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    }
    for key, default in PASSTHROUGH.items():
        value = os.environ.get(key, default)
        if value is not None:
            env[key] = value
    return env


def run(cmd, timeout=3600, quiet=False):
    # LXD nests exec results one level deeper than other operations: the return
    # code and the output paths are under metadata.metadata, not metadata.
    md = wait(
        req(
            "POST",
            f"/1.0/instances/{NAME}/exec",
            {
                "command": cmd,
                "wait-for-websocket": False,
                "record-output": True,
                "interactive": False,
                "environment": environment(),
            },
        ),
        timeout=timeout,
    )
    inner = md.get("metadata") or {}
    rc = inner.get("return", -1)
    out = ""
    for stream in ("1", "2"):
        p = (inner.get("output") or {}).get(stream)
        if p:
            b = req("GET", p, timeout=120)
            if isinstance(b, bytes):
                out += b.decode("utf-8", "replace")
    if not quiet:
        log(out.rstrip())
    return rc, out


def main():
    wants_meridian = any(m.strip() == "1" for m in os.environ.get("MERS", "0").split())

    binaries = [(SERVER_BIN, "/root/bin/bvc-server"), (BROADCAST_BIN, "/root/bin/broadcast")]
    if wants_meridian:
        binaries.append((MERIDIAN_BIN, "/root/bin/meridian"))

    for src, _ in binaries:
        if not os.path.exists(src):
            sys.exit(f"FATAL: missing {src}")
    for f in (f"{CERTS}/lxd-client.crt", f"{CA}/ca.crt", SCRIPT):
        if not os.path.exists(f):
            sys.exit(f"FATAL: missing {f}")

    log(f"== cleaning any previous {NAME} ==")
    destroy()

    log(f"== creating {NAME} ({CORES} cores, {MEM}, ubuntu {IMAGE}) ==")
    config = {"limits.cpu": CORES, "limits.memory": MEM}
    if os.environ.get("CT_PRIVILEGED"):
        config["security.privileged"] = "true"
    wait(
        req(
            "POST",
            "/1.0/instances",
            {
                "name": NAME,
                "type": "container",
                "ephemeral": False,
                "config": config,
                "source": {
                    "type": "image",
                    "protocol": "simplestreams",
                    "server": "https://cloud-images.ubuntu.com/releases",
                    "alias": IMAGE,
                },
            },
        )
    )
    wait(req("PUT", f"/1.0/instances/{NAME}/state", {"action": "start", "timeout": 60}))

    try:
        log("== waiting for network ==")
        for i in range(60):
            time.sleep(2)
            rc, _ = run(["sh", "-c", "getent hosts archive.ubuntu.com >/dev/null 2>&1"], quiet=True)
            if rc == 0:
                log(f"  network up after {2 * (i + 1)}s")
                break
        else:
            log("  WARNING: no DNS; apt will fail and so will anything needing it")

        log("== installing deps ==")
        run(
            [
                "sh",
                "-c",
                "apt-get update -qq && apt-get install -y -qq openssl curl python3 >/dev/null 2>&1; "
                "echo openssl=$(command -v openssl) curl=$(command -v curl) python3=$(command -v python3)",
            ]
        )

        log("== pushing binaries ==")
        run(["mkdir", "-p", "/root/bin"], quiet=True)
        for src, dst in binaries:
            push(src, dst)
        push(f"{CA}/ca.crt", "/root/bin/ca.crt", 0o644)
        push(f"{CA}/ca.key", "/root/bin/ca.key", 0o600)
        push(SCRIPT, "/root/bench.sh", strip_cr=True)

        log(f"== running {os.path.basename(SCRIPT)} ==")
        rc, _ = run(["bash", "/root/bench.sh"], timeout=3600)
        log(f"benchmark rc={rc}")
        return rc
    finally:
        log("== destroying container ==")
        destroy()


if __name__ == "__main__":
    sys.exit(0 if main() == 0 else 1)
