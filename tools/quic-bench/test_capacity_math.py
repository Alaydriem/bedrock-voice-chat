import json
import unittest

from capacity_math import (
    DELAY_BUCKETS,
    FAIL,
    INVALID,
    PASS,
    CapacitySearch,
    CellResult,
    MergedHistogram,
    cpu_ids,
    drifted,
    expected_frames,
    loss,
    parse_tagged,
    verdict,
)


def histogram(counts=None, overflow=0, negative=0):
    buckets = [0] * DELAY_BUCKETS
    for ms, count in (counts or {}).items():
        buckets[ms] = count
    return {"buckets": buckets, "overflow": overflow, "negative": negative}


def cell(histogram_obj, players=10, speakers=10, **overrides):
    values = dict(
        players=players,
        speakers=speakers,
        alive=players,
        stats_files=players,
        srv_cpu=14.0,
        load_cpu=40.0,
        load_cores=6,
        parse_failures=0,
        histogram=histogram_obj,
        expected=histogram_obj.received,
    )
    values.update(overrides)
    return CellResult(**values)


class MergedHistogramTest(unittest.TestCase):
    def test_merge_sums_buckets_overflow_and_negative(self):
        merged = MergedHistogram.merge(
            [histogram({5: 2}, overflow=1), histogram({5: 3, 9: 1}, negative=2)]
        )
        self.assertEqual(merged.buckets[5], 5)
        self.assertEqual(merged.buckets[9], 1)
        self.assertEqual(merged.overflow, 1)
        self.assertEqual(merged.negative, 2)
        self.assertEqual(merged.received, 7)

    def test_merge_rejects_wrong_bucket_count(self):
        with self.assertRaises(ValueError):
            MergedHistogram.merge([{"buckets": [0] * 10, "overflow": 0, "negative": 0}])

    def test_p99_is_the_bucket_holding_the_99th_percentile_rank(self):
        merged = MergedHistogram.merge([histogram({10: 99, 50: 1})])
        self.assertEqual(merged.percentile(0.99), 10)
        self.assertEqual(merged.percentile(1.0), 50)

    def test_p99_reports_bucket_count_when_rank_falls_in_overflow(self):
        merged = MergedHistogram.merge([histogram(overflow=100)])
        self.assertEqual(merged.percentile(0.99), DELAY_BUCKETS)
        self.assertEqual(merged.max_ms(), DELAY_BUCKETS)

    def test_percentile_of_empty_histogram_is_none(self):
        merged = MergedHistogram.merge([histogram()])
        self.assertIsNone(merged.percentile(0.99))
        self.assertIsNone(merged.max_ms())


class ExpectedAndLossTest(unittest.TestCase):
    def test_expected_counts_every_speaker_except_self(self):
        # 3 speakers receive 2 each, 2 listeners receive 3 each: 12 streams = s * (p - 1)
        self.assertEqual(expected_frames(5, 3, 20, 60), 12 * 50 * 60)

    def test_loss_is_clamped_at_zero_when_more_arrive_than_expected(self):
        self.assertEqual(loss(110, 100), 0.0)
        self.assertAlmostEqual(loss(98, 100), 0.02)

    def test_loss_with_no_expected_frames_is_zero(self):
        self.assertEqual(loss(0, 0), 0.0)


class VerdictTest(unittest.TestCase):
    def setUp(self):
        self.fast = MergedHistogram.merge([histogram({8: 1000})])

    def test_fast_and_complete_passes(self):
        self.assertEqual(verdict(cell(self.fast))[0], PASS)

    def test_p99_at_limit_fails(self):
        slow = MergedHistogram.merge([histogram({40: 1000})])
        self.assertEqual(verdict(cell(slow))[0], FAIL)

    def test_loss_at_one_percent_fails(self):
        result = verdict(cell(self.fast, expected=1000 * 100 // 99 + 1))
        self.assertEqual(result[0], FAIL)

    def test_all_late_frames_fail_without_error(self):
        late = MergedHistogram.merge([histogram(overflow=1000)])
        self.assertEqual(verdict(cell(late))[0], FAIL)

    def test_nothing_received_fails(self):
        empty = MergedHistogram.merge([histogram()])
        self.assertEqual(verdict(cell(empty, expected=5000))[0], FAIL)

    def test_negative_delay_means_clocks_not_shared(self):
        skewed = MergedHistogram.merge([histogram({8: 1000}, negative=1)])
        self.assertEqual(verdict(cell(skewed))[0], INVALID)

    def test_missing_stats_file_is_invalid_not_loss(self):
        self.assertEqual(verdict(cell(self.fast, stats_files=9))[0], INVALID)

    def test_dead_client_is_invalid(self):
        self.assertEqual(verdict(cell(self.fast, alive=9))[0], INVALID)

    def test_parse_failures_are_invalid(self):
        self.assertEqual(verdict(cell(self.fast, parse_failures=3))[0], INVALID)

    def test_load_generator_over_half_its_allowance_is_invalid(self):
        self.assertEqual(verdict(cell(self.fast, load_cpu=301.0, load_cores=6))[0], INVALID)
        self.assertEqual(verdict(cell(self.fast, load_cpu=300.0, load_cores=6))[0], PASS)


class CapacitySearchTest(unittest.TestCase):
    def run_search(self, search, capacity):
        tried = []
        n = search.next_n()
        while n is not None:
            tried.append(n)
            search.report(n, n <= capacity)
            n = search.next_n()
        return tried

    def test_coarse_up_then_fine_up(self):
        search = CapacitySearch(10)
        tried = self.run_search(search, 20)
        self.assertEqual(tried, [10, 14, 18, 22, 19, 20, 21])
        self.assertEqual(search.result, 20)

    def test_first_cell_failing_walks_down(self):
        search = CapacitySearch(10)
        tried = self.run_search(search, 4)
        self.assertEqual(tried, [10, 6, 2, 3, 4, 5])
        self.assertEqual(search.result, 4)

    def test_floor_failing_reports_none(self):
        search = CapacitySearch(10, floor=2)
        tried = self.run_search(search, 1)
        self.assertEqual(tried, [10, 6, 2])
        self.assertIsNone(search.result)

    def test_ceiling_stops_the_climb(self):
        search = CapacitySearch(10, ceiling=20)
        tried = self.run_search(search, 1000)
        self.assertEqual(tried, [10, 14, 18, 20])
        self.assertEqual(search.result, 20)


class CpuIdsTest(unittest.TestCase):
    def test_ranges_and_lists(self):
        self.assertEqual(cpu_ids("2-4,7"), {2, 3, 4, 7})
        self.assertEqual(cpu_ids("1"), {1})

    def test_overlap_is_detectable(self):
        self.assertIn(3, cpu_ids("2-7"))

    def test_malformed_spec_raises(self):
        with self.assertRaises(ValueError):
            cpu_ids("2-x")


class DriftTest(unittest.TestCase):
    def test_within_tolerance(self):
        self.assertFalse(drifted(14.0, 16.0))

    def test_beyond_tolerance(self):
        self.assertTrue(drifted(14.0, 17.5))

    def test_zero_baseline_is_drift(self):
        self.assertTrue(drifted(0.0, 1.0))


class ParseTaggedTest(unittest.TestCase):
    def test_collects_tagged_lines_and_ignores_the_rest(self):
        stats = json.dumps(histogram({1: 2}))
        out = f"noise\nALIVE 10\nSTATS 1 {stats}\nSTATS 2 {stats}\nLOADCPU 4012\n"
        tags = parse_tagged(out)
        self.assertEqual(tags["ALIVE"], ["10"])
        self.assertEqual(len(tags["STATS"]), 2)
        self.assertEqual(tags["STATS"][0].split(" ", 1)[0], "1")
        self.assertNotIn("noise", tags)


if __name__ == "__main__":
    unittest.main()
