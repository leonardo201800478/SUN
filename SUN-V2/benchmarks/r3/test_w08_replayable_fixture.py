"""Replayable W08 transaction fixture for exactly-once commit and retry.

This fixture runs independent P-A and R2 transaction transitions over the
same declarative input sequence. It is a synthetic model comparison, not a
physical SMS bus trace or production transaction implementation.
"""
from __future__ import annotations

import sys
import unittest
from fractions import Fraction
from pathlib import Path

R2_REFERENCE_PATH = Path(__file__).resolve().parents[2] / "verification" / "r2_reference"
sys.path.insert(0, str(R2_REFERENCE_PATH))

from p_a_transactions import (  # noqa: E402
    PAMarker,
    PAMarkerKind,
    PATransaction,
    PATransactionBook,
    PATransactionSpec,
    PATxState,
    commit_effect as p_a_commit_effect,
    cancel as p_a_cancel,
    consume_base_time as p_a_consume_base_time,
    consume_tw as p_a_consume_tw,
    complete as p_a_complete,
    enter_waiting as p_a_enter_waiting,
    fail as p_a_fail,
    reach_next_marker as p_a_reach_next_marker,
    recover as p_a_recover,
    release_waiting as p_a_release_waiting,
    sample_marker as p_a_sample_marker,
    start as p_a_start,
    submit as p_a_submit,
)
from replayable_fixture_corpus import (  # noqa: E402
    capture_differential_fixture,
    replay_fixture_side,
    record_replayable_fixture,
)
from transaction_reference import (  # noqa: E402
    Marker,
    MarkerKind,
    Transaction,
    TransactionBook,
    TransactionSpec,
    TxState,
    commit_effect as r2_commit_effect,
    cancel as r2_cancel,
    consume_base_time as r2_consume_base_time,
    consume_tw as r2_consume_tw,
    complete as r2_complete,
    enter_waiting as r2_enter_waiting,
    fail as r2_fail,
    reach_next_marker as r2_reach_next_marker,
    recover as r2_recover,
    release_waiting as r2_release_waiting,
    sample_marker as r2_sample_marker,
    start as r2_start,
    submit as r2_submit,
)


def _fraction(value):
    """Restore exact rational values encoded by the replay harness."""
    if isinstance(value, dict) and set(value) == {"$fraction"}:
        numerator, denominator = value["$fraction"]
        return Fraction(numerator, denominator)
    return value


def _spec_data(spec):
    """Serialize a transaction descriptor into the shared neutral schema."""
    return {
        "identity": spec.identity,
        "generation": spec.generation,
        "origin": spec.origin,
        "destination": spec.destination,
        "operation": spec.operation,
        "profile": spec.profile,
        "payload": spec.payload,
        "admitted_at": spec.admitted_at,
        "start_at": spec.start_at,
        "duration_base": spec.duration_base,
        "automatic_tws": spec.automatic_tws,
        "external_tws": spec.external_tws,
        "markers": [
            {
                "identity": item.identity,
                "kind": item.kind.value,
                "at_common": item.at_common,
                "irreversible": item.irreversible,
            }
            for item in spec.markers
        ],
    }


def _transaction_data(tx):
    """Serialize every transaction control field and its effect ledger."""
    return {
        "spec": _spec_data(tx.spec),
        "state": tx.state.value,
        "progress_common": tx.progress_common,
        "base_consumed": tx.base_consumed,
        "automatic_tw_consumed": tx.automatic_tw_consumed,
        "external_tw_consumed": tx.external_tw_consumed,
        "ready_marker": tx.ready_marker,
        "wait_source": tx.wait_source,
        "sampled": [list(item) for item in tx.sampled],
        "committed": list(tx.committed),
        "failure": tx.failure,
        "recoveries": tx.recoveries,
    }


def _book_data(book):
    """Serialize the complete transaction book in stable sequence order."""
    return {"transactions": [_transaction_data(tx) for tx in book.transactions]}


def _result_data(result):
    """Capture operation result, complete book, transaction and sampled value."""
    return {
        "code": result.code.value,
        "book": _book_data(result.book),
        "transaction": (
            _transaction_data(result.transaction)
            if result.transaction is not None else None
        ),
        "value": result.value,
        "detail": result.detail,
    }


def _p_a_spec(data):
    """Build a P-A-owned descriptor from one neutral declarative descriptor."""
    markers = tuple(
        PAMarker(
            item["identity"],
            PAMarkerKind(item["kind"]),
            _fraction(item["at_common"]),
            item["irreversible"],
        )
        for item in data["markers"]
    )
    return PATransactionSpec(
        data["identity"], data["generation"], data["origin"],
        data["destination"], data["operation"], data["profile"],
        data["payload"], _fraction(data["admitted_at"]),
        _fraction(data["start_at"]), _fraction(data["duration_base"]),
        data["automatic_tws"], data["external_tws"], markers,
    )


def _r2_spec(data):
    """Build an R2-owned descriptor without sharing P-A model types."""
    markers = tuple(
        Marker(
            item["identity"],
            MarkerKind(item["kind"]),
            _fraction(item["at_common"]),
            item["irreversible"],
        )
        for item in data["markers"]
    )
    return TransactionSpec(
        data["identity"], data["generation"], data["origin"],
        data["destination"], data["operation"], data["profile"],
        data["payload"], _fraction(data["admitted_at"]),
        _fraction(data["start_at"]), _fraction(data["duration_base"]),
        data["automatic_tws"], data["external_tws"], markers,
    )


def _book_payload(data):
    """Unwrap an operation result to the next full book state."""
    if "code" in data and "book" in data:
        return data["book"]
    return data


def _p_a_book(data):
    """Reconstruct P-A state using only P-A transaction types."""
    payload = _book_payload(data)
    transactions = []
    for item in payload["transactions"]:
        transactions.append(PATransaction(
            spec=_p_a_spec(item["spec"]),
            state=PATxState(item["state"]),
            progress_common=_fraction(item["progress_common"]),
            base_consumed=_fraction(item["base_consumed"]),
            automatic_tw_consumed=item["automatic_tw_consumed"],
            external_tw_consumed=item["external_tw_consumed"],
            ready_marker=item["ready_marker"],
            wait_source=item["wait_source"],
            sampled=tuple(tuple(value) for value in item["sampled"]),
            committed=tuple(item["committed"]),
            failure=item["failure"],
            recoveries=item["recoveries"],
        ))
    return PATransactionBook(tuple(transactions))


def _r2_book(data):
    """Reconstruct R2 state using only independent R2 reference types."""
    payload = _book_payload(data)
    transactions = []
    for item in payload["transactions"]:
        transactions.append(Transaction(
            spec=_r2_spec(item["spec"]),
            state=TxState(item["state"]),
            progress_common=_fraction(item["progress_common"]),
            base_consumed=_fraction(item["base_consumed"]),
            automatic_tw_consumed=item["automatic_tw_consumed"],
            external_tw_consumed=item["external_tw_consumed"],
            ready_marker=item["ready_marker"],
            wait_source=item["wait_source"],
            sampled=tuple(tuple(value) for value in item["sampled"]),
            committed=tuple(item["committed"]),
            failure=item["failure"],
            recoveries=item["recoveries"],
        ))
    return TransactionBook(tuple(transactions))


def _p_a_step(data, name, args):
    """Dispatch one operation exclusively through P-A transition functions."""
    book = _p_a_book(data)
    if name == "submit":
        result = p_a_submit(book, _p_a_spec(args["spec"]))
    elif name == "start":
        result = p_a_start(book, args["identity"])
    elif name == "reach_next_marker":
        result = p_a_reach_next_marker(
            book, args["identity"], _fraction(args["at_common"])
        )
    elif name == "commit_effect":
        result = p_a_commit_effect(book, args["identity"], args["marker_id"])
    elif name == "enter_waiting":
        result = p_a_enter_waiting(book, args["identity"], args["source"])
    elif name == "release_waiting":
        result = p_a_release_waiting(book, args["identity"], args["source"])
    elif name == "sample_marker":
        result = p_a_sample_marker(
            book, args["identity"], args["marker_id"], args["sampled_value"]
        )
    elif name == "consume_base_time":
        result = p_a_consume_base_time(book, args["identity"], _fraction(args["amount"]))
    elif name == "consume_tw":
        result = p_a_consume_tw(book, args["identity"], automatic=args["automatic"])
    elif name == "cancel":
        result = p_a_cancel(book, args["identity"])
    elif name == "fail":
        result = p_a_fail(book, args["identity"], args["error"])
    elif name == "recover":
        result = p_a_recover(book, args["identity"])
    elif name == "complete":
        result = p_a_complete(book, args["identity"])
    else:
        raise ValueError(f"unsupported W08 operation: {name}")
    return _result_data(result)


def _r2_step(data, name, args):
    """Dispatch one operation exclusively through independent R2 functions."""
    book = _r2_book(data)
    if name == "submit":
        result = r2_submit(book, _r2_spec(args["spec"]))
    elif name == "start":
        result = r2_start(book, args["identity"])
    elif name == "reach_next_marker":
        result = r2_reach_next_marker(
            book, args["identity"], _fraction(args["at_common"])
        )
    elif name == "commit_effect":
        result = r2_commit_effect(book, args["identity"], args["marker_id"])
    elif name == "enter_waiting":
        result = r2_enter_waiting(book, args["identity"], args["source"])
    elif name == "release_waiting":
        result = r2_release_waiting(book, args["identity"], args["source"])
    elif name == "sample_marker":
        result = r2_sample_marker(
            book, args["identity"], args["marker_id"], args["sampled_value"]
        )
    elif name == "consume_base_time":
        result = r2_consume_base_time(book, args["identity"], _fraction(args["amount"]))
    elif name == "consume_tw":
        result = r2_consume_tw(book, args["identity"], automatic=args["automatic"])
    elif name == "cancel":
        result = r2_cancel(book, args["identity"])
    elif name == "fail":
        result = r2_fail(book, args["identity"], args["error"])
    elif name == "recover":
        result = r2_recover(book, args["identity"])
    elif name == "complete":
        result = r2_complete(book, args["identity"])
    else:
        raise ValueError(f"unsupported W08 operation: {name}")
    return _result_data(result)


class W08ReplayableTransactionTests(unittest.TestCase):
    def test_w08_commit_is_captured_and_retry_is_exactly_once(self):
        spec = {
            "identity": "w08-commit",
            "generation": 0,
            "origin": "cpu",
            "destination": "vdp",
            "operation": "write",
            "profile": "SMS-NTSC",
            "payload": "0x5a",
            "admitted_at": Fraction(0),
            "start_at": Fraction(0),
            "duration_base": Fraction(0),
            "automatic_tws": 0,
            "external_tws": 0,
            "markers": (
                {
                    "identity": "write-commit",
                    "kind": "Commit",
                    "at_common": Fraction(3),
                    "irreversible": False,
                },
            ),
        }
        operations = (
            {"sequence": 0, "name": "submit", "args": {"spec": spec}},
            {"sequence": 1, "name": "start", "args": {"identity": "w08-commit"}},
            {
                "sequence": 2,
                "name": "reach_next_marker",
                "args": {"identity": "w08-commit", "at_common": Fraction(3)},
            },
            {
                "sequence": 3,
                "name": "commit_effect",
                "args": {"identity": "w08-commit", "marker_id": "write-commit"},
            },
            {
                "sequence": 4,
                "name": "commit_effect",
                "args": {"identity": "w08-commit", "marker_id": "write-commit"},
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W08_COMMIT_EXACTLY_ONCE_RETRY",
            provenance={
                "gate": "R3",
                "workload": "W08",
                "classification": "OBSERVATION",
                "scenario": "commit marker followed by identical retry",
                "scope": "synthetic P-A/R2 transaction model comparison",
                "limitation": (
                    "Not a physical SMS bus trace, hardware timing measurement, "
                    "or production transaction implementation."
                ),
            },
            initial_state={"transactions": []},
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        final_left = fixture["cuts"][-1]["left_state"]
        final_right = fixture["cuts"][-1]["right_state"]
        self.assertEqual(final_left["code"], "Ok")
        self.assertEqual(final_right["code"], "Ok")
        self.assertEqual(
            final_left["transaction"]["committed"], ["write-commit"]
        )
        self.assertEqual(
            final_right["transaction"]["committed"], ["write-commit"]
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )


    def test_w08_wait_resume_and_sample_retry_preserve_first_value(self):
        spec = {
            "identity": "w08-sample",
            "generation": 1,
            "origin": "cpu",
            "destination": "vdp",
            "operation": "read",
            "profile": "SMS-NTSC",
            "payload": "status",
            "admitted_at": Fraction(0),
            "start_at": Fraction(0),
            "duration_base": Fraction(0),
            "automatic_tws": 0,
            "external_tws": 0,
            "markers": (
                {
                    "identity": "read-status",
                    "kind": "Sample",
                    "at_common": Fraction(2),
                    "irreversible": False,
                },
            ),
        }
        operations = (
            {"sequence": 0, "name": "submit", "args": {"spec": spec}},
            {"sequence": 1, "name": "start", "args": {"identity": "w08-sample"}},
            {
                "sequence": 2,
                "name": "enter_waiting",
                "args": {"identity": "w08-sample", "source": "dependency"},
            },
            {
                "sequence": 3,
                "name": "release_waiting",
                "args": {"identity": "w08-sample", "source": "dependency"},
            },
            {
                "sequence": 4,
                "name": "reach_next_marker",
                "args": {"identity": "w08-sample", "at_common": Fraction(2)},
            },
            {
                "sequence": 5,
                "name": "sample_marker",
                "args": {
                    "identity": "w08-sample",
                    "marker_id": "read-status",
                    "sampled_value": "0x44",
                },
            },
            {
                "sequence": 6,
                "name": "sample_marker",
                "args": {
                    "identity": "w08-sample",
                    "marker_id": "read-status",
                    "sampled_value": "0x99",
                },
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W08_WAIT_RESUME_SAMPLE_EXACTLY_ONCE",
            provenance={
                "gate": "R3",
                "workload": "W08",
                "classification": "OBSERVATION",
                "scenario": "dependency WAIT, release and idempotent sample retry",
                "scope": "synthetic P-A/R2 transaction model comparison",
                "limitation": (
                    "Not a physical SMS WAIT signal, bus sample, hardware trace, "
                    "or production transaction implementation."
                ),
            },
            initial_state={"transactions": []},
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        final_left = fixture["cuts"][-1]["left_state"]
        final_right = fixture["cuts"][-1]["right_state"]
        self.assertEqual(final_left["code"], "Ok")
        self.assertEqual(final_right["code"], "Ok")
        self.assertEqual(final_left["value"], "0x44")
        self.assertEqual(final_right["value"], "0x44")
        self.assertEqual(
            final_left["transaction"]["sampled"], [["read-status", "0x44"]]
        )
        self.assertEqual(
            final_right["transaction"]["sampled"], [["read-status", "0x44"]]
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )


    def test_w08_failed_retry_requires_explicit_recovery(self):
        spec = {
            "identity": "w08-failure",
            "generation": 0,
            "origin": "cpu",
            "destination": "vdp",
            "operation": "write",
            "profile": "SMS-NTSC",
            "payload": "0xa5",
            "admitted_at": Fraction(0),
            "start_at": Fraction(0),
            "duration_base": Fraction(0),
            "automatic_tws": 0,
            "external_tws": 0,
            "markers": (),
        }
        operations = (
            {"sequence": 0, "name": "submit", "args": {"spec": spec}},
            {"sequence": 1, "name": "start", "args": {"identity": "w08-failure"}},
            {
                "sequence": 2,
                "name": "fail",
                "args": {"identity": "w08-failure", "error": "bus fault"},
            },
            {"sequence": 3, "name": "submit", "args": {"spec": spec}},
            {"sequence": 4, "name": "recover", "args": {"identity": "w08-failure"}},
            {"sequence": 5, "name": "complete", "args": {"identity": "w08-failure"}},
        )
        fixture = capture_differential_fixture(
            fixture_id="W08_FAILURE_RETRY_EXPLICIT_RECOVERY",
            provenance={
                "gate": "R3",
                "workload": "W08",
                "classification": "OBSERVATION",
                "scenario": "failed request retry does not recover; recovery is explicit",
                "scope": "synthetic P-A/R2 transaction model comparison",
                "limitation": (
                    "Not a physical bus failure, hardware recovery trace, "
                    "or production transaction implementation."
                ),
            },
            initial_state={"transactions": []},
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        retry_state = fixture["cuts"][3]["left_state"]["transaction"]
        final_left = fixture["cuts"][-1]["left_state"]
        final_right = fixture["cuts"][-1]["right_state"]
        self.assertEqual(retry_state["state"], "Failed")
        self.assertEqual(retry_state["recoveries"], 0)
        self.assertEqual(final_left["code"], "Ok")
        self.assertEqual(final_right["code"], "Ok")
        self.assertEqual(final_left["transaction"]["state"], "Completed")
        self.assertEqual(final_right["transaction"]["state"], "Completed")
        self.assertEqual(final_left["transaction"]["recoveries"], 1)
        self.assertEqual(final_right["transaction"]["recoveries"], 1)
        self.assertEqual(final_left["transaction"]["failure"], "bus fault")
        self.assertEqual(final_right["transaction"]["failure"], "bus fault")
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )




    def test_w08_base_duration_automatic_tw_and_external_tw_are_independent(self):
        spec = {
            "identity": "w08-tw-budget", "generation": 2,
            "origin": "cpu", "destination": "vdp", "operation": "write",
            "profile": "SMS-NTSC", "payload": "0x3c",
            "admitted_at": Fraction(0), "start_at": Fraction(0),
            "duration_base": Fraction(2), "automatic_tws": 1, "external_tws": 1,
            "markers": ({"identity": "write-commit", "kind": "Commit",
                         "at_common": Fraction(1), "irreversible": False},),
        }
        operations = (
            {"sequence": 0, "name": "submit", "args": {"spec": spec}},
            {"sequence": 1, "name": "start", "args": {"identity": "w08-tw-budget"}},
            {"sequence": 2, "name": "complete", "args": {"identity": "w08-tw-budget"}},
            {"sequence": 3, "name": "reach_next_marker", "args": {
                "identity": "w08-tw-budget", "at_common": Fraction(1)}},
            {"sequence": 4, "name": "commit_effect", "args": {
                "identity": "w08-tw-budget", "marker_id": "write-commit"}},
            {"sequence": 5, "name": "consume_base_time", "args": {
                "identity": "w08-tw-budget", "amount": Fraction(2)}},
            {"sequence": 6, "name": "consume_tw", "args": {
                "identity": "w08-tw-budget", "automatic": True}},
            {"sequence": 7, "name": "complete", "args": {"identity": "w08-tw-budget"}},
            {"sequence": 8, "name": "consume_tw", "args": {
                "identity": "w08-tw-budget", "automatic": False}},
            {"sequence": 9, "name": "complete", "args": {"identity": "w08-tw-budget"}},
        )
        fixture = capture_differential_fixture(
            fixture_id="W08_BASE_AUTOMATIC_TW_EXTERNAL_TW_SEPARATION",
            provenance={
                "gate": "R3", "workload": "W08", "classification": "OBSERVATION",
                "scenario": "base duration, automatic TW and external TW consumed independently",
                "scope": "synthetic P-A/R2 transaction model comparison",
                "limitation": "Not evidence of physical Z80 TW behavior, WAIT timing or SMS bus trace.",
            },
            initial_state={"transactions": []}, operations=operations,
            left_step=_p_a_step, right_step=_r2_step,
        )
        record_replayable_fixture(fixture)
        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        before_external_tw = fixture["cuts"][7]["left_state"]
        self.assertEqual(before_external_tw["code"], "Blocked")
        tx = before_external_tw["transaction"]
        self.assertEqual(tx["base_consumed"], {"$fraction": [2, 1]})
        self.assertEqual(tx["automatic_tw_consumed"], 1)
        self.assertEqual(tx["external_tw_consumed"], 0)
        for final in (fixture["cuts"][-1]["left_state"], fixture["cuts"][-1]["right_state"]):
            self.assertEqual(final["code"], "Ok")
            self.assertEqual(final["transaction"]["state"], "Completed")
            self.assertEqual(final["transaction"]["automatic_tw_consumed"], 1)
            self.assertEqual(final["transaction"]["external_tw_consumed"], 1)
        self.assertEqual(len(replay_fixture_side(fixture, step=_p_a_step, side="left")), len(operations))
        self.assertEqual(len(replay_fixture_side(fixture, step=_r2_step, side="right")), len(operations))

    def test_w08_cancel_before_irreversible_effect_and_reject_after_commit(self):
        cancel_spec = {
            "identity": "w08-cancel-before", "generation": 3,
            "origin": "cpu", "destination": "vdp", "operation": "write",
            "profile": "SMS-NTSC", "payload": "0x11",
            "admitted_at": Fraction(0), "start_at": Fraction(0),
            "duration_base": Fraction(0), "automatic_tws": 0, "external_tws": 0,
            "markers": (),
        }
        committed_spec = {
            "identity": "w08-cancel-after", "generation": 4,
            "origin": "cpu", "destination": "vdp", "operation": "write",
            "profile": "SMS-NTSC", "payload": "0x22",
            "admitted_at": Fraction(0), "start_at": Fraction(0),
            "duration_base": Fraction(0), "automatic_tws": 0, "external_tws": 0,
            "markers": ({"identity": "irreversible-write", "kind": "Commit",
                         "at_common": Fraction(1), "irreversible": True},),
        }
        operations = (
            {"sequence": 0, "name": "submit", "args": {"spec": cancel_spec}},
            {"sequence": 1, "name": "start", "args": {"identity": "w08-cancel-before"}},
            {"sequence": 2, "name": "cancel", "args": {"identity": "w08-cancel-before"}},
            {"sequence": 3, "name": "submit", "args": {"spec": cancel_spec}},
            {"sequence": 4, "name": "submit", "args": {"spec": committed_spec}},
            {"sequence": 5, "name": "start", "args": {"identity": "w08-cancel-after"}},
            {"sequence": 6, "name": "reach_next_marker", "args": {
                "identity": "w08-cancel-after", "at_common": Fraction(1)}},
            {"sequence": 7, "name": "commit_effect", "args": {
                "identity": "w08-cancel-after", "marker_id": "irreversible-write"}},
            {"sequence": 8, "name": "cancel", "args": {"identity": "w08-cancel-after"}},
        )
        fixture = capture_differential_fixture(
            fixture_id="W08_CANCEL_BOUNDARY_AND_LATE_INPUT",
            provenance={
                "gate": "R3", "workload": "W08", "classification": "OBSERVATION",
                "scenario": "cancel before irreversible commit; reject cancellation after commit",
                "scope": "synthetic P-A/R2 transaction model comparison",
                "limitation": "Synthetic cancellation boundary, not physical SMS bus rollback or device cancellation evidence.",
            },
            initial_state={"transactions": []}, operations=operations,
            left_step=_p_a_step, right_step=_r2_step,
        )
        record_replayable_fixture(fixture)
        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        cancelled_retry = fixture["cuts"][3]["left_state"]
        self.assertEqual(cancelled_retry["code"], "Ok")
        self.assertEqual(cancelled_retry["transaction"]["state"], "Cancelled")
        for final in (fixture["cuts"][-1]["left_state"], fixture["cuts"][-1]["right_state"]):
            self.assertEqual(final["code"], "LateInput")
            self.assertEqual(final["transaction"]["state"], "InProgress")
            self.assertEqual(final["transaction"]["committed"], ["irreversible-write"])
        self.assertEqual(len(replay_fixture_side(fixture, step=_p_a_step, side="left")), len(operations))
        self.assertEqual(len(replay_fixture_side(fixture, step=_r2_step, side="right")), len(operations))


    def test_w08_failure_after_partial_commit_preserves_exact_ledger(self):
        spec = {
            "identity": "w08-partial-failure",
            "generation": 0,
            "origin": "cpu",
            "destination": "vdp",
            "operation": "write",
            "profile": "SMS-NTSC",
            "payload": "0x11,0x22",
            "admitted_at": Fraction(0),
            "start_at": Fraction(0),
            "duration_base": Fraction(0),
            "automatic_tws": 0,
            "external_tws": 0,
            "markers": (
                {
                    "identity": "write-1",
                    "kind": "Commit",
                    "at_common": Fraction(1),
                    "irreversible": False,
                },
                {
                    "identity": "write-2",
                    "kind": "Commit",
                    "at_common": Fraction(2),
                    "irreversible": False,
                },
            ),
        }
        operations = (
            {"sequence": 0, "name": "submit", "args": {"spec": spec}},
            {
                "sequence": 1,
                "name": "start",
                "args": {"identity": "w08-partial-failure"},
            },
            {
                "sequence": 2,
                "name": "reach_next_marker",
                "args": {
                    "identity": "w08-partial-failure",
                    "at_common": Fraction(1),
                },
            },
            {
                "sequence": 3,
                "name": "commit_effect",
                "args": {
                    "identity": "w08-partial-failure",
                    "marker_id": "write-1",
                },
            },
            {
                "sequence": 4,
                "name": "fail",
                "args": {
                    "identity": "w08-partial-failure",
                    "error": "partial bus fault",
                },
            },
        )
        fixture = capture_differential_fixture(
            fixture_id="W08_FAILURE_AFTER_PARTIAL_COMMIT_PRESERVES_LEDGER",
            provenance={
                "gate": "R3",
                "workload": "W08",
                "classification": "OBSERVATION",
                "scenario": (
                    "failure after the first of two commit markers preserves "
                    "the exact already-committed effect ledger"
                ),
                "scope": "synthetic P-A/R2 transaction model comparison",
                "limitation": (
                    "Not a physical partial bus failure, hardware recovery trace, "
                    "or production transaction implementation."
                ),
            },
            initial_state={"transactions": []},
            operations=operations,
            left_step=_p_a_step,
            right_step=_r2_step,
        )
        record_replayable_fixture(fixture)

        self.assertTrue(fixture["all_equal"])
        self.assertIsNone(fixture["first_divergence"])
        self.assertEqual(len(fixture["cuts"]), len(operations))
        for side in ("left_state", "right_state"):
            final = fixture["cuts"][-1][side]
            self.assertEqual(final["code"], "Ok")
            self.assertEqual(final["transaction"]["state"], "Failed")
            self.assertEqual(final["transaction"]["committed"], ["write-1"])
            self.assertEqual(final["transaction"]["failure"], "partial bus fault")
            self.assertEqual(final["transaction"]["recoveries"], 0)
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_p_a_step, side="left")),
            len(operations),
        )
        self.assertEqual(
            len(replay_fixture_side(fixture, step=_r2_step, side="right")),
            len(operations),
        )

if __name__ == "__main__":
    unittest.main(verbosity=2)
