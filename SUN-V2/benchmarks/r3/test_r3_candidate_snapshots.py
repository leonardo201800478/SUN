"""W09 candidate-local snapshot/restore/replay tests for disposable P-D/P-E."""
from __future__ import annotations

import unittest
from dataclasses import replace
from fractions import Fraction

from p_d_discrete_agenda import (
    AgendaContext, AgendaParticipant, AgendaState,
    arbitrate_cancellations as pd_arbitrate_cancellations,
    close_watermark as pd_close_watermark,
    publish_with_context as pd_publish_with_context,
    request_cancel as pd_request_cancel,
)
from p_e_sync_on_demand import (
    SyncCode, SyncDomain, SyncState, SyncTransition, synchronize,
)
from r3_candidate_snapshots import (
    SnapshotCode,
    capture,
    make_replay_event,
    replay_from_snapshot,
    restore,
)
from r3_transaction_core import (
    MarkerKind, TransactionBook, TxMarker, TxSpec,
    reach_next_marker, sample_marker, start, submit,
)


def partial_transaction_book():
    spec = TxSpec(
        "tx-snapshot", 3, "cpu", "vdp", "read", "SMS-NTSC", "payload",
        Fraction(0), Fraction(0), Fraction(2), 1, 1,
        (
            TxMarker("sample-status", MarkerKind.SAMPLE, Fraction(1)),
            TxMarker("commit-write", MarkerKind.COMMIT, Fraction(2), True),
        ),
    )
    book = submit(TransactionBook(), spec).book
    book = start(book, "tx-snapshot").book
    book = reach_next_marker(book, "tx-snapshot", Fraction(1)).book
    book = sample_marker(book, "tx-snapshot", "sample-status", "0x5A").book
    return book


def pending_pd_state():
    names = frozenset(("cpu", "vdp"))
    context = AgendaContext(
        (
            AgendaParticipant("cpu", Fraction(0), Fraction(100)),
            AgendaParticipant("vdp", Fraction(0), Fraction(100)),
        ),
        names,
    )
    state = pd_publish_with_context(
        AgendaState(), context, owner="cpu", identity="deadline-snapshot",
        generation=2, at_common=Fraction(10), effect_id="device-write",
    ).state
    state = pd_request_cancel(
        state, context, owner="cpu", identity="deadline-snapshot",
        generation=2, at_common=Fraction(8),
    ).state
    state = pd_close_watermark(state, Fraction(9), context).state
    return replace(state, transactions=partial_transaction_book())


class CandidateSnapshotTests(unittest.TestCase):
    def test_p_d_snapshot_round_trips_pending_cancel_heap_and_w08_ledger(self):
        state = pending_pd_state()
        result = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        )
        self.assertEqual(result.code, SnapshotCode.OK)
        restored = restore(
            AgendaState(), result.artifact,
            expected_candidate="P-D", expected_profile="SMS-NTSC",
            expected_version="P-D-R3-W09-1",
        )
        self.assertEqual(restored.code, SnapshotCode.OK)
        self.assertEqual(restored.restored_state, state)
        self.assertEqual(restored.restored_state.heap, state.heap)
        self.assertEqual(restored.restored_state.pending_cancellations, state.pending_cancellations)
        self.assertEqual(restored.restored_state.transactions, state.transactions)
        self.assertEqual(restored.restored_state.transactions.transactions[0].sampled,
                         (("sample-status", "0x5A"),))

    def test_p_d_snapshot_replay_arbitrates_then_closes_deterministically(self):
        state = pending_pd_state()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        ).artifact
        context = AgendaContext(
            (
                AgendaParticipant("cpu", Fraction(0), Fraction(100)),
                AgendaParticipant("vdp", Fraction(0), Fraction(100)),
            ),
            frozenset(("cpu", "vdp")),
            Fraction(9),
        )

        def apply(current, operation, arguments):
            if operation == "arbitrate":
                result = pd_arbitrate_cancellations(current)
            elif operation == "close":
                result = pd_close_watermark(
                    current, arguments["target_common"], context
                )
            else:
                raise ValueError("unknown P-D replay operation")
            if result.code.value != "Accepted":
                raise ValueError(result.detail or result.code.value)
            return result.state

        expected = pd_arbitrate_cancellations(state).state
        expected = pd_close_watermark(expected, Fraction(10), context).state
        events = (
            make_replay_event(5, "close", {"target_common": 10}),
            make_replay_event(4, "arbitrate", {}),
        )
        replayed = replay_from_snapshot(
            artifact, events, expected_candidate="P-D",
            expected_profile="SMS-NTSC", expected_version="P-D-R3-W09-1",
            apply=apply,
        )
        self.assertEqual(replayed.code, SnapshotCode.OK)
        self.assertEqual(replayed.input_cursor, 5)
        self.assertEqual(replayed.state, expected)

    def test_p_e_snapshot_rebuilds_indexes_and_replays_observation_suffix(self):
        initial = SyncState(
            domains=(
                SyncDomain("cpu", Fraction(0), Fraction(10)),
                SyncDomain("vdp", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("cpu-write", "cpu", Fraction(2)),
                SyncTransition("vdp-read", "vdp", Fraction(4), ("cpu-write",)),
            ),
            transactions=partial_transaction_book(),
        )
        first = synchronize(
            initial, requested_domains=("cpu",), target_common=Fraction(2)
        )
        self.assertEqual(first.code, SyncCode.REACHED)
        state = first.state
        artifact = capture(
            "P-E", state, profile="SMS-NTSC",
            model_version="P-E-R3-W09-1", input_cursor=1,
        ).artifact
        restored = restore(
            SyncState(domains=state.domains, transitions=state.transitions,
                      constraints=state.constraints, observed=state.observed,
                      transactions=state.transactions),
            artifact,
            expected_candidate="P-E", expected_profile="SMS-NTSC",
            expected_version="P-E-R3-W09-1",
        )
        self.assertEqual(restored.code, SnapshotCode.OK)
        self.assertEqual(restored.restored_state, state)
        self.assertTrue(restored.restored_state._validated)
        self.assertIn("vdp-read", restored.restored_state._transition_by_id)
        self.assertEqual(restored.restored_state.transactions, state.transactions)

        def apply(current, operation, arguments):
            if operation != "synchronize":
                raise ValueError("unknown P-E replay operation")
            result = synchronize(
                current,
                requested_domains=tuple(arguments["requested_domains"]),
                target_common=Fraction(arguments["target_common"]),
            )
            if result.code is not SyncCode.REACHED:
                raise ValueError(result.detail or result.code.value)
            return result.state

        expected = synchronize(
            state, requested_domains=("vdp",), target_common=Fraction(4)
        )
        replayed = replay_from_snapshot(
            artifact,
            (make_replay_event(
                2, "synchronize",
                {"requested_domains": ["vdp"], "target_common": 4},
            ),),
            expected_candidate="P-E", expected_profile="SMS-NTSC",
            expected_version="P-E-R3-W09-1", apply=apply,
        )
        self.assertEqual(expected.code, SyncCode.REACHED)
        self.assertEqual(replayed.code, SnapshotCode.OK)
        self.assertEqual(replayed.input_cursor, 2)
        self.assertEqual(replayed.state, expected.state)

    def test_restore_rejects_tampered_or_wrong_profile_artifact_atomically(self):
        state = pending_pd_state()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        ).artifact
        active = AgendaState()
        tampered = replace(artifact, digest="0" * 64)
        invalid = restore(
            active, tampered, expected_candidate="P-D",
            expected_profile="SMS-NTSC", expected_version="P-D-R3-W09-1",
        )
        self.assertEqual(invalid.code, SnapshotCode.INCOMPATIBLE)
        self.assertIs(invalid.active_state, active)
        self.assertIsNone(invalid.restored_state)

        wrong_profile = restore(
            active, artifact, expected_candidate="P-D",
            expected_profile="SG-1000", expected_version="P-D-R3-W09-1",
        )
        self.assertEqual(wrong_profile.code, SnapshotCode.INCOMPATIBLE)
        self.assertIs(wrong_profile.active_state, active)
        self.assertIsNone(wrong_profile.restored_state)

    def test_replay_rejects_profile_or_candidate_mismatch_before_apply(self):
        state = pending_pd_state()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        ).artifact
        applied = []
        result = replay_from_snapshot(
            artifact, (make_replay_event(4, "noop", {}),),
            expected_candidate="P-D", expected_profile="SG-1000",
            expected_version="P-D-R3-W09-1",
            apply=lambda current, operation, arguments: applied.append(operation) or current,
        )
        self.assertEqual(result.code, SnapshotCode.INCOMPATIBLE)
        self.assertEqual(result.state, state)
        self.assertEqual(result.input_cursor, 3)
        self.assertEqual(applied, [])

    def test_replay_sequence_gap_is_rejected_without_partial_apply(self):
        state = pending_pd_state()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        ).artifact
        applied = []

        def apply(current, operation, arguments):
            applied.append(operation)
            return current

        replayed = replay_from_snapshot(
            artifact,
            (
                make_replay_event(4, "noop", {}),
                make_replay_event(6, "noop", {}),
            ),
            expected_candidate="P-D", expected_profile="SMS-NTSC",
            expected_version="P-D-R3-W09-1", apply=apply,
        )
        self.assertEqual(replayed.code, SnapshotCode.INVALID_INPUT)
        self.assertEqual(replayed.input_cursor, 3)
        self.assertEqual(replayed.state, state)
        self.assertEqual(applied, [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
