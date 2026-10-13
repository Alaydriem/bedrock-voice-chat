#!/usr/bin/env python3
"""Find players-per-host on one pinned, contended CPU.

bvcsut runs the server on PIN_CPU, bvcnoise runs stress-ng on the same CPU, and
bvcload runs every client on LOAD_CPUS. See tools/quic-bench/README.md.
"""
import json
import os
import sys

from capacity_math import (
    INVALID,
    PASS,
    CapacitySearch,
    CellResult,
    MergedHistogram,
    cpu_ids,
    drifted,
    expected_frames,
    parse_tagged,
    verdict,
)
from lxd_api import LxdClient, LxdInstance

HERE = os.path.dirname(__file__)
HOST = os.environ.get("LXD_HOST", "https://10.57.2.3:8443")
CERTS = os.environ.get("LXD_CERTS", os.path.expanduser("~/.config/bvc-quic-bench"))
CA = os.environ.get("BVC_CA", "server/server/certificates")
IMAGE = os.environ.get("CT_IMAGE", "24.04")
TARGET = os.environ.get("BVC_TARGET_DIR", os.path.expanduser("~/bvc-target"))
SERVER_BIN = os.environ.get("BVC_SERVER_BIN", f"{TARGET}/x86_64-unknown-linux-musl/release/bvc-server")
BROADCAST_BIN = os.environ.get("BVC_BROADCAST_BIN", f"{TARGET}/release/examples/broadcast")

PIN_CPU = int(os.environ.get("PIN_CPU", "1"))
LOAD_CPUS = os.environ.get("LOAD_CPUS", "2-7")
NOISE_LEVELS = [int(n) for n in os.environ.get("NOISE", "0 50 80").split()]
N_START = int(os.environ.get("N_START", "10"))
N_CEILING = int(os.environ.get("N_CEILING", "64"))
WINDOW = int(os.environ.get("WINDOW", "60"))
FRAME_MS = int(os.environ.get("FRAME_MS", "20"))
CONTROL = (10, 10)
SMOKE = (3, 3)

BOTS = "/root/bench/bots"
IDENTITY_FILES = ("test.crt", "test.key", "ca.crt")


def log(*a):
    print(*a, flush=True)


def environment():
    env = {
        "HOME": "/root",
        "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "WINDOW": str(WINDOW),
        "FRAME_MS": str(FRAME_MS),
    }
    if os.environ.get("BATCH_WAIT_MICROS") is not None:
        env["BATCH_WAIT_MICROS"] = os.environ["BATCH_WAIT_MICROS"]
    return env


class CapacityRun:
    def __init__(self, client):
        self.env = environment()
        self.sut = LxdInstance(client, "bvcsut")
        self.noise = LxdInstance(client, "bvcnoise")
        self.load = LxdInstance(client, "bvcload")

    def provision(self):
        pin = f"{PIN_CPU}-{PIN_CPU}"
        for instance, config, packages in (
            (self.sut, {"limits.cpu": pin, "limits.memory": "2GB"}, ["openssl", "curl", "python3"]),
            (self.noise, {"limits.cpu": pin, "limits.memory": "512MB"}, ["stress-ng"]),
            (self.load, {"limits.cpu": LOAD_CPUS, "limits.memory": "6GB"}, ["python3"]),
        ):
            log(f"== creating {instance.name} ({config['limits.cpu']}) ==")
            instance.destroy()
            instance.create(config, IMAGE)
            instance.start()
            if not instance.wait_for_network(self.env):
                raise RuntimeError(f"{instance.name}: no network")
            instance.apt_install(packages, self.env)

        lib = os.path.join(HERE, "bench-in-container.sh")
        self.sut.run(["mkdir", "-p", "/root/bin"], self.env, quiet=True)
        self.sut.push(SERVER_BIN, "/root/bin/bvc-server")
        self.sut.push(f"{CA}/ca.crt", "/root/bin/ca.crt", 0o644)
        self.sut.push(f"{CA}/ca.key", "/root/bin/ca.key", 0o600)
        self.sut.push(lib, "/root/bench-lib.sh", strip_cr=True)
        self.sut.push(os.path.join(HERE, "capacity-server.sh"), "/root/capacity-server.sh", strip_cr=True)

        self.load.run(["mkdir", "-p", "/root/bin"], self.env, quiet=True)
        self.load.push(BROADCAST_BIN, "/root/bin/broadcast")
        self.load.push(lib, "/root/bench-lib.sh", strip_cr=True)
        self.load.push(os.path.join(HERE, "capacity-load.sh"), "/root/capacity-load.sh", strip_cr=True)
        sut_ip = self.sut.ipv4()
        self.load.run(["sh", "-c", f"echo '{sut_ip} bvcsut' >> /etc/hosts"], self.env, quiet=True)
        log(f"  bvcsut is {sut_ip}")

    def set_noise(self, level):
        command = "pkill stress-ng; sleep 1"
        if level > 0:
            command += (
                f"; stress-ng --cpu 1 --cpu-load {level} --timeout 0 "
                "> /dev/null 2>&1 < /dev/null &"
            )
        self.noise.run(["sh", "-c", command], self.env, quiet=True)

    def copy_identities(self, players):
        dirs = " ".join(f"{BOTS}/{i}" for i in range(1, players + 1))
        self.load.run(["sh", "-c", f"rm -rf {BOTS} && mkdir -p {dirs}"], self.env, quiet=True)
        for i in range(1, players + 1):
            for name in IDENTITY_FILES:
                data = self.sut.pull(f"{BOTS}/{i}/{name}")
                self.load.push_bytes(data, f"{BOTS}/{i}/{name}", 0o600)

    def sut_command(self, *args):
        rc, out = self.sut.run(["bash", "/root/capacity-server.sh", *map(str, args)], self.env, quiet=True)
        if rc != 0:
            raise RuntimeError(f"capacity-server.sh {args[0]} failed:\n{out}")
        return parse_tagged(out), out

    def load_command(self, *args):
        rc, out = self.load.run(["bash", "/root/capacity-load.sh", *map(str, args)], self.env, quiet=True)
        if rc != 0:
            raise RuntimeError(f"capacity-load.sh {args[0]} failed:\n{out}")
        return parse_tagged(out)

    def cell(self, players, speakers):
        try:
            _, out = self.sut_command("setup", players)
            if "READY" not in out:
                raise RuntimeError(f"server setup did not finish:\n{out}")
            self.copy_identities(players)
            start_ms, end_ms = self.load_command("start", players, speakers)["WINDOW"][0].split()
            srv_tags, _ = self.sut_command("measure", start_ms, WINDOW)
            load_tags = self.load_command("collect", players, end_ms)
        finally:
            self.sut.run(["bash", "/root/capacity-server.sh", "stop"], self.env, quiet=True)

        srv = srv_tags["SRVCPU"][0].split()
        stats = [json.loads(line.split(" ", 1)[1]) for line in load_tags.get("STATS", [])]
        histogram = MergedHistogram.merge(stats)
        return CellResult(
            players=players,
            speakers=speakers,
            alive=int(load_tags["ALIVE"][0]),
            stats_files=len(stats),
            srv_cpu=int(srv[0]) / 100,
            load_cpu=int(load_tags["LOADCPU"][0]) / 100,
            load_cores=int(load_tags["CORES"][0]),
            parse_failures=int(srv[4]),
            histogram=histogram,
            expected=expected_frames(players, speakers, FRAME_MS, WINDOW),
        )

    def judged_cell(self, noise, players, speakers):
        for attempt in (1, 2):
            result = self.cell(players, speakers)
            outcome, reason = verdict(result)
            self.print_cell(noise, result, outcome, reason)
            if outcome != INVALID:
                return outcome, result
            if attempt == 1:
                log("   retrying once")
        return INVALID, result

    def print_cell(self, noise, r, outcome, reason):
        h = r.histogram
        lost = 0.0 if r.expected == 0 else max(0.0, 1 - h.received / r.expected)
        log(
            f"noise={noise:<3} {r.players}p/{r.speakers}s  srv={r.srv_cpu:6.2f}% "
            f"load={r.load_cpu:6.2f}% p50={h.percentile(0.5)}ms p99={h.percentile(0.99)}ms "
            f"max={h.max_ms()}ms loss={lost:.2%} {outcome} {reason}"
        )

    def search(self, noise, speakers_for):
        search = CapacitySearch(N_START, ceiling=N_CEILING)
        n = search.next_n()
        while n is not None:
            outcome, _ = self.judged_cell(noise, n, speakers_for(n))
            if outcome == INVALID:
                return f"INVALID at {n}"
            search.report(n, outcome == PASS)
            n = search.next_n()
        return "none" if search.result is None else str(search.result)


def main():
    if PIN_CPU in cpu_ids(LOAD_CPUS):
        sys.exit(f"FATAL: PIN_CPU {PIN_CPU} is inside LOAD_CPUS {LOAD_CPUS}")
    for path in (SERVER_BIN, BROADCAST_BIN, f"{CERTS}/lxd-client.crt", f"{CA}/ca.crt"):
        if not os.path.exists(path):
            sys.exit(f"FATAL: missing {path}")

    run = CapacityRun(LxdClient(HOST, CERTS))
    try:
        run.provision()
        run.set_noise(0)

        smoke_outcome, _ = run.judged_cell(0, *SMOKE)
        if smoke_outcome != PASS:
            log("FATAL: smoke cell did not pass; the measurement is wrong, not the server")
            return 1
        control_outcome, control_before = run.judged_cell(0, *CONTROL)
        if control_outcome == INVALID:
            log("FATAL: control cell INVALID")
            return 1
        if os.environ.get("SMOKE"):
            return 0

        table = []
        for noise in NOISE_LEVELS:
            run.set_noise(noise)
            all_talking = run.search(noise, lambda n: n)
            sixty = run.search(noise, lambda n: max(1, round(0.6 * n)))
            table.append((noise, all_talking, sixty))

        run.set_noise(0)
        _, control_after = run.judged_cell(0, *CONTROL)
        drift = drifted(control_before.srv_cpu, control_after.srv_cpu)

        log("")
        log("| Noise | All talking | 60% talking |")
        log("| --- | --- | --- |")
        for noise, all_talking, sixty in table:
            log(f"| {noise}% | {all_talking} | {sixty} |")
        log(
            f"control srv CPU: {control_before.srv_cpu:.2f}% before, "
            f"{control_after.srv_cpu:.2f}% after"
        )
        if drift:
            log("RUN INVALID: control cell drifted by more than 20%; the host load changed")
            return 1
        return 0
    finally:
        if not os.environ.get("KEEP"):
            for instance in (run.load, run.noise, run.sut):
                instance.destroy()


if __name__ == "__main__":
    sys.exit(main())
