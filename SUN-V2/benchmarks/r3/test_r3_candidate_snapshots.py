"""W09 candidate-local snapshot/restore/replay tests for disposable P-D/P-E."""
from __future__ import annotations

import hashlib
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
    PairRelation as SyncPairRelation, SyncCode, SyncConstraint, SyncDomain,
    SyncState, SyncTransition, synchronize,
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
    cancel, complete, consume_base_time, consume_tw, enter_waiting, fail,
    reach_next_marker, recover, sample_marker, start, submit,
)


def partial_transaction_book():
    def accepted(result):
        if result.code.value != "Ok":
            raise AssertionError(result.detail or result.code.value)
        return result.book

    markers = (
        TxMarker("sample-status", MarkerKind.SAMPLE, Fraction(1)),
        TxMarker("commit-write", MarkerKind.COMMIT, Fraction(2), True),
    )
    spec = TxSpec(
        "tx-snapshot", 3, "cpu", "vdp", "read", "SMS-NTSC", "payload",
        Fraction(0), Fraction(0), Fraction(2), 1, 1, markers,
    )
    book = accepted(submit(TransactionBook(), spec))
    book = accepted(start(book, "tx-snapshot"))
    book = accepted(reach_next_marker(book, "tx-snapshot", Fraction(1)))
    book = accepted(sample_marker(book, "tx-snapshot", "sample-status", "0x5A"))
    book = accepted(consume_base_time(book, "tx-snapshot", Fraction(1)))
    book = accepted(consume_tw(book, "tx-snapshot", automatic=True))
    book = accepted(consume_tw(book, "tx-snapshot", automatic=False))

    waiting = TxSpec(
        "tx-waiting", 0, "cpu", "vdp", "read", "SMS-NTSC", "",
        Fraction(0), Fraction(0), Fraction(0), 0, 0, (),
    )
    book = accepted(submit(book, waiting))
    book = accepted(start(book, "tx-waiting"))
    book = accepted(enter_waiting(book, "tx-waiting", "dependency"))

    failed = TxSpec(
        "tx-failed", 1, "cpu", "vdp", "write", "SMS-NTSC", "partial",
        Fraction(0), Fraction(0), Fraction(2), 0, 0, markers,
    )
    book = accepted(submit(book, failed))
    book = accepted(start(book, "tx-failed"))
    book = accepted(reach_next_marker(book, "tx-failed", Fraction(1)))
    book = accepted(sample_marker(book, "tx-failed", "sample-status", "0xA5"))
    book = accepted(reach_next_marker(book, "tx-failed", Fraction(2)))
    book = accepted(complete_commit_marker(book, "tx-failed"))
    book = accepted(fail(book, "tx-failed", "device failure after partial commit"))

    recovered = TxSpec(
        "tx-recovered", 0, "cpu", "vdp", "read", "SMS-NTSC", "",
        Fraction(0), Fraction(0), Fraction(0), 0, 0, (),
    )
    book = accepted(submit(book, recovered))
    book = accepted(start(book, "tx-recovered"))
    book = accepted(fail(book, "tx-recovered", "transient transport failure"))
    book = accepted(recover(book, "tx-recovered"))

    cancelled = TxSpec(
        "tx-cancelled", 0, "cpu", "vdp", "read", "SMS-NTSC", "",
        Fraction(0), Fraction(0), Fraction(0), 0, 0, (),
    )
    book = accepted(submit(book, cancelled))
    book = accepted(cancel(book, "tx-cancelled"))

    finished = TxSpec(
        "tx-completed", 0, "cpu", "vdp", "write", "SMS-NTSC", "",
        Fraction(0), Fraction(0), Fraction(0), 0, 0,
        (
            TxMarker("sample-zero", MarkerKind.SAMPLE, Fraction(0)),
            TxMarker("commit-zero", MarkerKind.COMMIT, Fraction(0), True),
        ),
    )
    book = accepted(submit(book, finished))
    book = accepted(start(book, "tx-completed"))
    book = accepted(reach_next_marker(book, "tx-completed", Fraction(0)))
    book = accepted(sample_marker(book, "tx-completed", "sample-zero", "zero"))
    book = accepted(reach_next_marker(book, "tx-completed", Fraction(0)))
    book = accepted(commit_effect(book, "tx-completed", "commit-zero"))
    book = accepted(complete(book, "tx-completed"))
    return book


def complete_commit_marker(book, identity):
    from r3_transaction_core import commit_effect
    return commit_effect(book, identity, "commit-write")


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
    state = pd_publish_with_context(
        state, context, owner="vdp", identity="future-deadline",
        generation=0, at_common=Fraction(12), effect_id="future-effect",
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
        repeated = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        )
        self.assertEqual(result.code, SnapshotCode.OK)
        self.assertEqual(result.artifact.payload, repeated.artifact.payload)
        self.assertEqual(result.artifact.digest, repeated.artifact.digest)
        restored = restore(
            AgendaState(), result.artifact,
            expected_candidate="P-D", expected_profile="SMS-NTSC",
            expected_version="P-D-R3-W09-1",
        )
        self.assertEqual(restored.code, SnapshotCode.OK)
        self.assertEqual(restored.restored_state, state)
        self.assertTrue(state.heap)
        self.assertEqual(restored.restored_state.heap, state.heap)
        self.assertIn((Fraction(12), "future-deadline", 0), restored.restored_state.heap)
        self.assertEqual(restored.restored_state.pending_cancellations, state.pending_cancellations)
        self.assertEqual(restored.restored_state.transactions, state.transactions)
        transactions = {
            item.spec.identity: item for item in restored.restored_state.transactions.transactions
        }
        self.assertEqual(transactions["tx-snapshot"].sampled,
                         (("sample-status", "0x5A"),))
        self.assertEqual(transactions["tx-snapshot"].base_consumed, Fraction(1))
        self.assertEqual(transactions["tx-snapshot"].automatic_tw_consumed, 1)
        self.assertEqual(transactions["tx-snapshot"].external_tw_consumed, 1)
        self.assertEqual(transactions["tx-waiting"].wait_source, "dependency")
        self.assertEqual(transactions["tx-waiting"].state.value, "Waiting")
        self.assertEqual(transactions["tx-failed"].committed, ("commit-write",))
        self.assertEqual(transactions["tx-failed"].failure, "device failure after partial commit")
        self.assertEqual(transactions["tx-recovered"].recoveries, 1)
        self.assertEqual(transactions["tx-recovered"].failure, "transient transport failure")
        self.assertEqual(transactions["tx-cancelled"].state.value, "Cancelled")
        self.assertEqual(transactions["tx-completed"].state.value, "Completed")

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
                SyncDomain("psg", Fraction(0), Fraction(10)),
            ),
            transitions=(
                SyncTransition("cpu-write", "cpu", Fraction(2)),
                SyncTransition("vdp-read", "vdp", Fraction(4), ("cpu-write",)),
                SyncTransition("psg-a", "psg", Fraction(3)),
                SyncTransition("psg-b", "psg", Fraction(3)),
            ),
            constraints=(
                SyncConstraint("psg-a", "psg-b", SyncPairRelation.COMMUTATIVE),
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
        group_key = frozenset(("psg-a", "psg-b"))
        self.assertIn(group_key, restored.restored_state._commutative_groups)
        self.assertEqual(restored.restored_state.constraints, state.constraints)
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

    def test_snapshot_rejects_boolean_input_cursor_even_with_recomputed_digest(self):
        state = pending_pd_state()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=1,
        ).artifact
        payload = artifact.payload.replace(b'"input_cursor":1', b'"input_cursor":true')
        self.assertNotEqual(payload, artifact.payload)
        forged = replace(
            artifact, payload=payload, digest=hashlib.sha256(payload).hexdigest()
        )
        active = AgendaState()
        result = restore(
            active, forged, expected_candidate="P-D",
            expected_profile="SMS-NTSC", expected_version="P-D-R3-W09-1",
        )
        self.assertEqual(result.code, SnapshotCode.INCOMPATIBLE)
        self.assertIs(result.active_state, active)
        self.assertIsNone(result.restored_state)

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

    def test_replay_failure_after_valid_prefix_returns_original_restored_state(self):
        state = pending_pd_state()
        artifact = capture(
            "P-D", state, profile="SMS-NTSC",
            model_version="P-D-R3-W09-1", input_cursor=3,
        ).artifact
        applied = []

        def apply(current, operation, arguments):
            applied.append(operation)
            if operation == "arbitrate":
                result = pd_arbitrate_cancellations(current)
                if result.code.value != "Accepted":
                    raise ValueError(result.detail)
                return result.state
            raise ValueError("deliberate replay suffix failure")

        replayed = replay_from_snapshot(
            artifact,
            (
                make_replay_event(4, "arbitrate", {}),
                make_replay_event(5, "deliberate-failure", {}),
            ),
            expected_candidate="P-D", expected_profile="SMS-NTSC",
            expected_version="P-D-R3-W09-1", apply=apply,
        )
        self.assertEqual(replayed.code, SnapshotCode.INVALID_INPUT)
        self.assertEqual(replayed.state, state)
        self.assertEqual(replayed.input_cursor, 3)
        self.assertEqual(applied, ["arbitrate", "deliberate-failure"])

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
