"""Arithmetic for the shared-CPU capacity bench.

No I/O and no LXD calls, so every rule that decides a result is unit-tested in
test_capacity_math.py.
"""
import math
from dataclasses import dataclass

DELAY_BUCKETS = 1000
P99_LIMIT_MS = 40
LOSS_LIMIT = 0.01
LOAD_CPU_LIMIT_FRACTION = 0.5
DRIFT_TOLERANCE = 0.2

PASS = "PASS"
FAIL = "FAIL"
INVALID = "INVALID"


class MergedHistogram:
    def __init__(self, buckets, overflow, negative):
        self.buckets = buckets
        self.overflow = overflow
        self.negative = negative

    @classmethod
    def merge(cls, histograms):
        buckets = [0] * DELAY_BUCKETS
        overflow = 0
        negative = 0
        for h in histograms:
            if len(h["buckets"]) != DELAY_BUCKETS:
                raise ValueError(
                    f"histogram has {len(h['buckets'])} buckets, expected {DELAY_BUCKETS}"
                )
            buckets = [a + b for a, b in zip(buckets, h["buckets"])]
            overflow += h["overflow"]
            negative += h["negative"]
        return cls(buckets, overflow, negative)

    @property
    def received(self):
        return sum(self.buckets) + self.overflow

    def percentile(self, q):
        total = self.received
        if total == 0:
            return None
        rank = math.ceil(q * total)
        seen = 0
        for ms, count in enumerate(self.buckets):
            seen += count
            if seen >= rank:
                return ms
        return DELAY_BUCKETS

    def max_ms(self):
        if self.overflow:
            return DELAY_BUCKETS
        for ms in range(DELAY_BUCKETS - 1, -1, -1):
            if self.buckets[ms]:
                return ms
        return None


def expected_frames(players, speakers, frame_ms, window_s):
    # Speakers hear every other speaker, listeners hear every speaker: s * (p - 1) streams.
    return speakers * (players - 1) * (1000 // frame_ms) * window_s


def loss(received, expected):
    if expected <= 0:
        return 0.0
    return max(0.0, 1.0 - received / expected)


@dataclass(frozen=True)
class CellResult:
    players: int
    speakers: int
    alive: int
    stats_files: int
    srv_cpu: float
    load_cpu: float
    load_cores: int
    parse_failures: int
    histogram: MergedHistogram
    expected: int


def verdict(cell):
    if cell.alive < cell.players:
        return INVALID, f"{cell.alive}/{cell.players} clients alive"
    if cell.stats_files < cell.players:
        return INVALID, f"{cell.stats_files}/{cell.players} stats files"
    if cell.parse_failures > 0:
        return INVALID, f"server failed to parse {cell.parse_failures} packets"
    if cell.histogram.negative > 0:
        return INVALID, f"{cell.histogram.negative} negative delays: clocks not shared"
    load_limit = cell.load_cores * 100 * LOAD_CPU_LIMIT_FRACTION
    if cell.load_cpu > load_limit:
        return INVALID, f"load generator at {cell.load_cpu:.1f}% > {load_limit:.0f}%"

    p99 = cell.histogram.percentile(0.99)
    lost = loss(cell.histogram.received, cell.expected)
    if p99 is None:
        return FAIL, "nothing received"
    if p99 >= P99_LIMIT_MS:
        return FAIL, f"p99 {p99}ms >= {P99_LIMIT_MS}ms"
    if lost >= LOSS_LIMIT:
        return FAIL, f"loss {lost:.2%} >= {LOSS_LIMIT:.0%}"
    return PASS, ""


class CapacitySearch:
    def __init__(self, start, step=4, floor=2, ceiling=64):
        self.start = start
        self.step = step
        self.floor = floor
        self.ceiling = ceiling
        self.best_pass = None
        self.lowest_fail = None

    def next_n(self):
        if self.best_pass is None and self.lowest_fail is None:
            return self.start
        if self.lowest_fail is None:
            if self.best_pass >= self.ceiling:
                return None
            return min(self.best_pass + self.step, self.ceiling)
        if self.best_pass is None:
            if self.lowest_fail <= self.floor:
                return None
            return max(self.lowest_fail - self.step, self.floor)
        n = self.best_pass + 1
        return n if n < self.lowest_fail else None

    def report(self, n, passed):
        if passed:
            self.best_pass = n if self.best_pass is None else max(self.best_pass, n)
        else:
            self.lowest_fail = n if self.lowest_fail is None else min(self.lowest_fail, n)

    @property
    def result(self):
        return self.best_pass


def cpu_ids(spec):
    ids = set()
    for part in spec.split(","):
        part = part.strip()
        if "-" in part:
            low, high = part.split("-", 1)
            ids.update(range(int(low), int(high) + 1))
        else:
            ids.add(int(part))
    return ids


def drifted(before, after, tolerance=DRIFT_TOLERANCE):
    if before <= 0:
        return True
    return abs(after - before) / before > tolerance


def parse_tagged(output):
    tags = {}
    for line in output.splitlines():
        head, _, rest = line.partition(" ")
        if head.isupper() and head.isalpha():
            tags.setdefault(head, []).append(rest)
    return tags
