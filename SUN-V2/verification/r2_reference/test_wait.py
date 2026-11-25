"""Discriminating WAIT-source and TW-boundary tests for R2-T09."""
import unittest

from wait_reference import (
    WaitCode, WaitSource, WaitState, complete_cycle, consume_automatic_tw,
    sample_external_wait, start_cycle,
)


class R2T09WaitTests(unittest.TestCase):
    def test_automatic_io_tw_is_not_an_external_wait_sample(self):
        cycle = start_cycle(WaitSource.NONE, 1).cycle
        cycle = consume_automatic_tw(cycle).cycle
        done = complete_cycle(cycle)
        self.assertEqual(done.code, WaitCode.OK)
        self.assertEqual(done.cycle.external_tws_consumed, 0)
        self.assertEqual(done.cycle.samples, ())

    def test_interrupt_ack_automatic_tw_budget_is_two(self):
        cycle = start_cycle(WaitSource.NONE, 2).cycle
        cycle = consume_automatic_tw(cycle).cycle
        cycle = consume_automatic_tw(cycle).cycle
        self.assertEqual(complete_cycle(cycle).code, WaitCode.OK)
        self.assertEqual(cycle.automatic_tws_consumed, 2)

    def test_external_wait_active_consumes_tw_and_resamples_each_tw(self):
        cycle = start_cycle(WaitSource.EXTERNAL, 0).cycle
        first = sample_external_wait(cycle, True)
        self.assertEqual(first.cycle.state, WaitState.WAITING)
        self.assertEqual(first.cycle.external_tws_consumed, 1)
        self.assertEqual(first.cycle.next_sample_point, "TW")
        second = sample_external_wait(first.cycle, True)
        self.assertEqual(second.cycle.external_tws_consumed, 2)
        released = sample_external_wait(second.cycle, False)
        self.assertEqual(released.cycle.state, WaitState.RUNNING)
        self.assertIsNone(released.cycle.next_sample_point)
        self.assertEqual(released.cycle.samples, (("T2", True), ("TW", True), ("TW", False)))
        self.assertEqual(complete_cycle(released.cycle).code, WaitCode.OK)

    def test_unknown_wait_blocks_without_consuming_tw_or_progress(self):
        cycle = start_cycle(WaitSource.UNKNOWN, 1).cycle
        blocked = sample_external_wait(cycle, None)
        self.assertEqual(blocked.code, WaitCode.BLOCKED)
        self.assertEqual(blocked.cycle, cycle)
        self.assertEqual(blocked.cycle.external_tws_consumed, 0)
        self.assertEqual(blocked.cycle.automatic_tws_consumed, 0)

    def test_unknown_source_is_not_reinterpreted_as_inactive(self):
        cycle = start_cycle(WaitSource.UNKNOWN, 0).cycle
        blocked = sample_external_wait(cycle, False)
        self.assertEqual(blocked.code, WaitCode.BLOCKED)
        self.assertEqual(blocked.cycle, cycle)

    def test_fixture_wait_uses_same_timing_but_keeps_fixture_provenance(self):
        cycle = start_cycle(WaitSource.FIXTURE, 0).cycle
        waiting = sample_external_wait(cycle, True)
        released = sample_external_wait(waiting.cycle, False)
        self.assertEqual(released.code, WaitCode.OK)
        self.assertEqual(released.cycle.source, WaitSource.FIXTURE)

    def test_automatic_tw_budget_cannot_be_exceeded(self):
        cycle = start_cycle(WaitSource.NONE, 1).cycle
        cycle = consume_automatic_tw(cycle).cycle
        overflow = consume_automatic_tw(cycle)
        self.assertEqual(overflow.code, WaitCode.INVALID_INPUT)
        self.assertEqual(overflow.cycle, cycle)

    def test_no_external_wait_profile_rejects_pin_sampling(self):
        cycle = start_cycle(WaitSource.NONE, 1).cycle
        invalid = sample_external_wait(cycle, True)
        self.assertEqual(invalid.code, WaitCode.INVALID_INPUT)
        self.assertEqual(invalid.cycle, cycle)

    def test_cycle_cannot_complete_while_wait_is_active(self):
        cycle = start_cycle(WaitSource.EXTERNAL, 0).cycle
        cycle = sample_external_wait(cycle, True).cycle
        blocked = complete_cycle(cycle)
        self.assertEqual(blocked.code, WaitCode.BLOCKED)
        self.assertEqual(blocked.cycle, cycle)

    def test_cycle_cannot_complete_before_automatic_tw_budget(self):
        cycle = start_cycle(WaitSource.NONE, 2).cycle
        cycle = consume_automatic_tw(cycle).cycle
        blocked = complete_cycle(cycle)
        self.assertEqual(blocked.code, WaitCode.BLOCKED)
        self.assertEqual(blocked.cycle, cycle)

    def test_releasing_external_wait_does_not_erase_automatic_tw_requirement(self):
        cycle = start_cycle(WaitSource.EXTERNAL, 1).cycle
        cycle = sample_external_wait(cycle, False).cycle
        blocked = complete_cycle(cycle)
        self.assertEqual(blocked.code, WaitCode.BLOCKED)
        cycle = consume_automatic_tw(cycle).cycle
        self.assertEqual(complete_cycle(cycle).code, WaitCode.OK)


if __name__ == "__main__":
    unittest.main(verbosity=2)
