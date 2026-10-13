#!/usr/bin/env python3
"""Drive a BVC voice-load benchmark inside a throwaway LXD container.

Creates the container over the LXD HTTPS API, pushes the prebuilt binaries and a
benchmark script into it, runs the script, streams the output back, then deletes
the container. Clients and server are co-resident, so all traffic is loopback.

Configuration is entirely by environment; see tools/quic-bench/README.md.
"""
import os
import sys

from lxd_api import LxdClient, LxdInstance

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

CLIENT = LxdClient(HOST, CERTS)
INSTANCE = LxdInstance(CLIENT, NAME)


def log(*a):
    print(*a, flush=True)


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

    env = environment()

    log(f"== cleaning any previous {NAME} ==")
    INSTANCE.destroy()

    log(f"== creating {NAME} ({CORES} cores, {MEM}, ubuntu {IMAGE}) ==")
    config = {"limits.cpu": CORES, "limits.memory": MEM}
    if os.environ.get("CT_PRIVILEGED"):
        config["security.privileged"] = "true"
    INSTANCE.create(config, IMAGE)
    INSTANCE.start()

    try:
        log("== waiting for network ==")
        if not INSTANCE.wait_for_network(env):
            log("  WARNING: no DNS; apt will fail and so will anything needing it")

        log("== installing deps ==")
        INSTANCE.apt_install(["openssl", "curl", "python3"], env)

        log("== pushing binaries ==")
        INSTANCE.run(["mkdir", "-p", "/root/bin"], env, quiet=True)
        for src, dst in binaries:
            INSTANCE.push(src, dst)
        INSTANCE.push(f"{CA}/ca.crt", "/root/bin/ca.crt", 0o644)
        INSTANCE.push(f"{CA}/ca.key", "/root/bin/ca.key", 0o600)
        INSTANCE.push(SCRIPT, "/root/bench.sh", strip_cr=True)

        log(f"== running {os.path.basename(SCRIPT)} ==")
        rc, _ = INSTANCE.run(["bash", "/root/bench.sh"], env, timeout=3600)
        log(f"benchmark rc={rc}")
        return rc
    finally:
        log("== destroying container ==")
        INSTANCE.destroy()


if __name__ == "__main__":
    sys.exit(0 if main() == 0 else 1)
