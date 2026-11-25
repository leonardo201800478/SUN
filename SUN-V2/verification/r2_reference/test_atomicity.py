"""Cross-subsystem error precedence and no-partial-mutation checks for R2-T15."""
import unittest
from fractions import Fraction

from chronos_reference import ClockDomain, advance, make_state
from deadline_reference import DeadlineBook, DeadlineCode, publish_deadline
from replay_reference import (
    ExternalInput, InputContract, Participant, ReplayCode, ReplayState,
    accept_input, advance_to,
)
from snapshot_reference import (
    AckCommit, AckPresence, InterruptRecord, MachineState, SnapshotCode,
    capture, restore,
)
from transaction_reference import (
    Marker, MarkerKind, TransactionBook, TransactionSpec, TxCode, cancel,
    commit_effect, reach_next_marker, start, submit,
)


def temporal_state(
    *, progress=0, watermark=None, domains=("cpu",), expected=None, horizon=100
):
    expected = tuple(expected if expected is not None else domains)
    clocks = tuple(
        ClockDomain(
            name, Fraction(1), progress_local=Fraction(progress),
            horizon_local=Fraction(horizon),
        )
        for name in domains
    )
    return make_state(
        profile="SMS-NTSC-ABSTRACT",
        domains=clocks,
        expected_participants=expected,
        watermark_common=None if watermark is None else Fraction(watermark),
    )


class R2T15CrossSubsystemAtomicityTests(unittest.TestCase):
    def test_invalid_deadline_descriptor_precedes_retrograde_time_classification(self):
        reference = temporal_state(progress=10)
        result = publish_deadline(
            DeadlineBook(), reference, owner="", identity="d", generation=0,
            at_common=Fraction(0), effect_id="effect:d:0",
        )
        self.assertEqual(result.code, DeadlineCode.INVALID_INPUT)
        self.assertEqual(result.book, DeadlineBook(watermark_common=Fraction(10)))

    def test_missing_participant_blocks_before_partial_late_frontier(self):
        replay_state = ReplayState(
            "SMS-NTSC",
            (Participant("cpu", Fraction(10), Fraction(20)),),
            (InputContract("port", ("cpu", "vdp")),),
            Fraction(0),
        )
        event = ExternalInput("e", "port", "cpu", Fraction(1), "write", ("cpu", "vdp"))
        result = accept_input(replay_state, event)
        self.assertEqual(result.code, ReplayCode.BLOCKED)
        self.assertEqual(result.state, replay_state)

    def test_invalid_clock_conversion_is_indeterminate_before_late_input(self):
        replay_state = ReplayState(
            "SMS-NTSC",
            (
                Participant("cpu", Fraction(10), Fraction(20)),
                Participant("vdp", Fraction(10), Fraction(20), False),
            ),
            (InputContract("port", ("cpu", "vdp")),),
            Fraction(0),
        )
        event = ExternalInput("e", "port", "cpu", Fraction(1), "write", ("cpu", "vdp"))
        result = accept_input(replay_state, event)
        self.assertEqual(result.code, ReplayCode.INDETERMINATE)
        self.assertEqual(result.state, replay_state)

    def test_retrograde_advance_and_internal_regression_have_distinct_results(self):
        initial = temporal_state(progress=5)
        retrograde = advance(initial, Fraction(4))
        regression_state = temporal_state(progress=5, horizon=4)
        regression = advance(regression_state, Fraction(6))
        self.assertEqual(retrograde.result.kind.value, "LateInput")
        self.assertEqual(regression.result.kind.value, "InvariantViolation")
        self.assertEqual(retrograde.state, initial)
        self.assertEqual(regression.state, regression_state)

    def test_closed_watermark_rejects_equal_deadline_even_with_microstep(self):
        reference = temporal_state(progress=0, watermark=10)
        result = publish_deadline(
            DeadlineBook(watermark_common=Fraction(10)), reference,
            owner="cpu", identity="d", generation=0,
            at_common=Fraction(10), effect_id="effect:d:0",
            microstep=True, microstep_level=1,
        )
        self.assertEqual(result.code, DeadlineCode.LATE_INPUT)
        self.assertEqual(result.book, DeadlineBook())

    def test_owner_progress_equal_deadline_without_microstep_is_indeterminate_on_open_cut(self):
        reference = temporal_state(progress=10, watermark=0)
        result = publish_deadline(
            DeadlineBook(), reference, owner="cpu", identity="d", generation=0,
            at_common=Fraction(10), effect_id="effect:d:0",
        )
        self.assertEqual(result.code, DeadlineCode.INDETERMINATE)
        self.assertEqual(result.book, DeadlineBook())

    def test_cancel_after_irreversible_commit_is_late_without_rollback(self):
        spec = TransactionSpec(
            "tx", 0, "cpu", "vdp", "write", "SMS-NTSC", "data",
            Fraction(0), Fraction(0), Fraction(0), 0, 0,
            (Marker("commit", MarkerKind.COMMIT, Fraction(2)),),
        )
        admitted = submit(TransactionBook(), spec)
        started = start(admitted.book, "tx")
        ready = reach_next_marker(started.book, "tx", Fraction(2))
        committed = commit_effect(ready.book, "tx", "commit")
        rejected = cancel(committed.book, "tx")
        self.assertEqual(rejected.code, TxCode.LATE_INPUT)
        self.assertEqual(rejected.book, committed.book)
        self.assertEqual(rejected.transaction.committed, ("commit",))

    def test_unknown_ack_window_has_no_snapshot_artifact_and_no_mutation(self):
        source = MachineState(
            "SMS-NTSC", "R1-0.3.18", (), Fraction(0), (), (),
            InterruptRecord(
                accepted="INT", acknowledge_active=True,
                device_effect_presence=AckPresence.UNKNOWN,
                device_effect_commit=AckCommit.UNKNOWN,
            ),
            0,
        )
        result = capture(source)
        self.assertEqual(result.code, SnapshotCode.UNSUPPORTED)
        self.assertIsNone(result.artifact)
        self.assertEqual(result.active_state, source)

    def test_incompatible_snapshot_restore_does_not_partially_replace_active_state(self):
        source = MachineState(
            "SMS-NTSC", "R1-0.3.18", (), Fraction(0), (), (), InterruptRecord(), 0
        )
        artifact = capture(source).artifact
        active = MachineState(
            "SMS-NTSC", "R1-0.3.18", (), Fraction(5), (), (), InterruptRecord(), 9
        )
        result = restore(
            active, artifact, expected_profile="SG-1000", expected_version="R1-0.3.18"
        )
        self.assertEqual(result.code, SnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.active_state, active)
        self.assertIsNone(result.restored_state)

    def test_same_frontier_retry_is_idempotent_but_new_same_time_input_is_late(self):
        replay_state = ReplayState(
            "SMS-NTSC",
            (Participant("cpu", Fraction(4), Fraction(10)),),
            (InputContract("port", ("cpu",)),),
            Fraction(4),
        )
        self.assertEqual(advance_to(replay_state, Fraction(4)).state, replay_state)
        event = ExternalInput("e", "port", "cpu", Fraction(4), "write", ("cpu",))
        rejected = accept_input(replay_state, event)
        self.assertEqual(rejected.code, ReplayCode.LATE_INPUT)
        self.assertEqual(rejected.state, replay_state)


if __name__ == "__main__":
    unittest.main(verbosity=2)
