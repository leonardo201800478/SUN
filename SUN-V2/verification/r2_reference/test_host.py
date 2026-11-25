"""R2-T13 host-output independence and chunk-invariance regressions."""
import unittest

from host_reference import (
    HostBlock, HostCode, VirtualState, advance_core, drop_host_block,
    drain_host, make_host, recreate_host_device, resize_host_buffer,
    set_vsync, signal_overrun, signal_underrun, submit_block,
)


class R2T13HostIndependenceTests(unittest.TestCase):
    def test_drain_does_not_change_virtual_time_or_digital_output(self):
        core = advance_core(VirtualState("SMS-NTSC"), 8)
        block = HostBlock("b1", 0, core.samples)
        host = submit_block(make_host(4), block).host
        drained = drain_host(host, 1)
        self.assertEqual(drained.host.delivered, (block,))
        self.assertEqual(core.state.virtual_ticks, 8)
        self.assertEqual(core.state.digital_samples, block.samples)

    def test_host_delay_and_queue_retention_do_not_change_core_state(self):
        core = advance_core(VirtualState("SMS-NTSC"), 5)
        host = submit_block(make_host(4), HostBlock("b", 0, core.samples)).host
        self.assertEqual(host.queue[0].samples, core.samples)
        self.assertEqual(core.state.sample_cursor, 5)

    def test_host_drop_does_not_rewind_core_sample_cursor(self):
        core = advance_core(VirtualState("SMS-NTSC"), 4)
        host = submit_block(make_host(2), HostBlock("b", 0, core.samples)).host
        dropped = drop_host_block(host, "b")
        self.assertEqual(dropped.host.dropped, ("b",))
        self.assertEqual(core.state.sample_cursor, 4)

    def test_duplicate_host_block_is_not_redelivered_or_regenerated(self):
        core = advance_core(VirtualState("SMS-NTSC"), 4)
        block = HostBlock("b", 0, core.samples)
        host = submit_block(make_host(2), block).host
        duplicate = submit_block(host, block)
        self.assertEqual(duplicate.code, HostCode.BLOCKED)
        self.assertEqual(duplicate.host, host)
        self.assertEqual(core.state.sample_cursor, 4)

    def test_underrun_and_overrun_are_host_only_counters(self):
        core = advance_core(VirtualState("SMS-NTSC"), 7)
        host = make_host(2)
        host = signal_underrun(host).host
        host = signal_overrun(host).host
        self.assertEqual((host.underruns, host.overruns), (1, 1))
        self.assertEqual(core.state.virtual_ticks, 7)

    def test_vsync_and_buffer_resize_do_not_change_virtual_state(self):
        core = advance_core(VirtualState("SMS-NTSC"), 6)
        host = make_host(2)
        host = set_vsync(host, True).host
        host = resize_host_buffer(host, 8).host
        self.assertTrue(host.vsync_enabled)
        self.assertEqual(host.buffer_limit, 8)
        self.assertEqual(core.state.sample_cursor, 6)

    def test_device_recreation_is_host_only(self):
        core = advance_core(VirtualState("SMS-NTSC"), 3)
        host = recreate_host_device(make_host(2)).host
        self.assertEqual(host.device_generation, 1)
        self.assertEqual(core.state.virtual_ticks, 3)

    def test_different_core_chunk_partitions_produce_identical_state_and_samples(self):
        whole = advance_core(VirtualState("SMS-NTSC"), 17)
        state = VirtualState("SMS-NTSC")
        chunks = []
        for size in (2, 3, 1, 4, 7):
            result = advance_core(state, size)
            state = result.state
            chunks.extend(result.samples)
        self.assertEqual(state, whole.state)
        self.assertEqual(tuple(chunks), whole.samples)

    def test_host_buffer_chunking_does_not_change_core_sample_sequence(self):
        core = advance_core(VirtualState("SMS-NTSC"), 12)
        first = HostBlock("a", 0, core.samples[:4])
        second = HostBlock("b", 4, core.samples[4:])
        host = make_host(4)
        host = submit_block(host, first).host
        host = submit_block(host, second).host
        self.assertEqual(tuple(value for block in host.queue for value in block.samples), core.samples)
        self.assertEqual(core.state.sample_cursor, 12)

    def test_host_reprocessing_never_changes_core_generated_output(self):
        core = advance_core(VirtualState("SMS-NTSC"), 9)
        block = HostBlock("b", 0, core.samples)
        host = submit_block(make_host(3), block).host
        host = drain_host(host, 1).host
        duplicate = submit_block(host, block)
        self.assertEqual(duplicate.code, HostCode.BLOCKED)
        self.assertEqual(core.state.digital_samples, block.samples)

    def test_full_host_queue_reports_overrun_without_touching_core(self):
        core = advance_core(VirtualState("SMS-NTSC"), 4)
        host = make_host(1)
        host = submit_block(host, HostBlock("a", 0, core.samples[:2])).host
        full = submit_block(host, HostBlock("b", 2, core.samples[2:]))
        self.assertEqual(full.code, HostCode.BLOCKED)
        self.assertEqual(full.host.overruns, 1)
        self.assertEqual(core.state.sample_cursor, 4)

    def test_invalid_host_operations_leave_host_state_unchanged(self):
        host = make_host(2)
        invalid = resize_host_buffer(host, 0)
        self.assertEqual(invalid.code, HostCode.INVALID_INPUT)
        self.assertEqual(invalid.host, host)


if __name__ == "__main__":
    unittest.main(verbosity=2)
