"""W09 executable matrix for the shared R3 TransactionBook -> R2 snapshot fields."""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "verification" / "r2_reference"))

from r3_transaction_core import (  # noqa: E402
    MarkerKind, Transaction, TransactionBook, TxMarker, TxSpec, TxState,
)
from snapshot_reference import (  # noqa: E402
    InterruptRecord, MachineState, SnapshotCode, TransactionState,
    capture, restore,
)
from transaction_snapshot_projection import (  # noqa: E402
    TransactionProjectionCode, project_transaction_book,
)


def candidate_book():
    marker = TxMarker("sample-next", MarkerKind.SAMPLE, Fraction(2))
    spec = TxSpec(
        identity="tx-shared",
        generation=4,
        origin="cpu",
        destination="vdp",
        operation="read",
        profile="SMS-NTSC",
        payload="request",
        admitted_at=Fraction(0),
        start_at=Fraction(1),
        duration_base=Fraction(5),
        automatic_tws=2,
        external_tws=3,
        markers=(marker,),
    )
    records = (
        Transaction(spec, TxState.CREATED, Fraction(0)),
        Transaction(spec, TxState.IN_PROGRESS, Fraction(1)),
        Transaction(spec, TxState.WAITING, Fraction(1), wait_source="external"),
        Transaction(spec, TxState.READY, Fraction(2), ready_marker="sample-next"),
        Transaction(spec, TxState.COMPLETED, Fraction(5)),
        Transaction(spec, TxState.CANCELLED, Fraction(1)),
        Transaction(spec, TxState.FAILED, Fraction(2), failure="device fault"),
    )
    # Use distinct stable IDs for each lifecycle row in the comparison matrix.
    return TransactionBook(tuple(
        Transaction(
            spec=TxSpec(
                identity=f"tx-{index}-{item.state.value.lower()}",
                generation=spec.generation,
                origin=spec.origin,
                destination=spec.destination,
                operation=spec.operation,
                profile=spec.profile,
                payload=spec.payload,
                admitted_at=spec.admitted_at,
                start_at=spec.start_at,
                duration_base=spec.duration_base,
                automatic_tws=spec.automatic_tws,
                external_tws=spec.external_tws,
                markers=spec.markers,
            ),
            state=item.state,
            progress_common=item.progress_common,
            ready_marker=item.ready_marker,
            wait_source=item.wait_source,
            failure=item.failure,
        )
        for index, item in enumerate(records)
    ))


class TransactionSnapshotProjectionTests(unittest.TestCase):
    def test_maps_all_shared_lifecycle_states_and_roundtrips_through_r2(self):
        projected = project_transaction_book(candidate_book())
        self.assertEqual(projected.code, TransactionProjectionCode.OK)
        expected_states = {
            TxState.CREATED: TransactionState.CREATED,
            TxState.IN_PROGRESS: TransactionState.IN_PROGRESS,
            TxState.WAITING: TransactionState.WAITING,
            TxState.READY: TransactionState.READY,
            TxState.COMPLETED: TransactionState.COMPLETED,
            TxState.CANCELLED: TransactionState.CANCELLED,
            TxState.FAILED: TransactionState.FAILED,
        }
        actual = {record.state for record in projected.projection.records}
        self.assertEqual(actual, set(expected_states.values()))

        state = MachineState(
            profile="SMS-NTSC",
            model_version="R1-0.3.18",
            domains=(),
            watermark=None,
            deadlines=(),
            transactions=projected.projection.records,
            interrupts=InterruptRecord(),
            input_cursor=0,
        )
        captured = capture(state)
        self.assertEqual(captured.code, SnapshotCode.OK)
        restored = restore(
            state, captured.artifact,
            expected_profile="SMS-NTSC", expected_version="R1-0.3.18",
        )
        self.assertEqual(restored.code, SnapshotCode.OK)
        self.assertEqual(restored.restored_state.transactions, projected.projection.records)

    def test_preserves_only_explicitly_shared_coordinates(self):
        projected = project_transaction_book(candidate_book()).projection
        waiting = next(item for item in projected.records if item.state is TransactionState.WAITING)
        ready = next(item for item in projected.records if item.state is TransactionState.READY)
        failed = next(item for item in projected.records if item.state is TransactionState.FAILED)
        self.assertEqual(waiting.wait_source, "external")
        self.assertEqual(ready.next_marker, "sample-next")
        self.assertEqual(failed.error, "device fault")
        self.assertTrue(any("committed marker IDs" in item for item in projected.unrepresented))
        self.assertTrue(any("pending_effects" in item for item in projected.unrepresented))
        self.assertTrue(any("TW budgets" in item for item in projected.unrepresented))

    def test_invalid_candidate_book_is_rejected_without_projection(self):
        invalid = TransactionBook((candidate_book().transactions[0],
                                   candidate_book().transactions[0]))
        result = project_transaction_book(invalid)
        self.assertEqual(result.code, TransactionProjectionCode.INVALID_INPUT)
        self.assertIsNone(result.projection)


if __name__ == "__main__":
    unittest.main(verbosity=2)
