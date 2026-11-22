import json
import tempfile
import unittest
from pathlib import Path

from compare_output_code_candidate import (
    candidate_output_code,
    characterize,
    iter_trace,
    load_predicted_sine,
    signed_measurement,
)


class OutputCodeCandidateTests(unittest.TestCase):
    def test_exhaustive_nine_bit_channel_domain_is_one_to_one(self):
        codes = [candidate_output_code(value) for value in range(-256, 256)]
        self.assertEqual(sorted(codes), list(range(512)))
        self.assertEqual(len(set(codes)), 512)

    def test_both_zero_encodings_remain_distinct(self):
        self.assertEqual(candidate_output_code(0), 255)
        self.assertEqual(candidate_output_code(-1), 256)
        self.assertEqual(signed_measurement(0), (0, False))
        self.assertEqual(signed_measurement(-1), (0, True))

    def test_full_scale_endpoints(self):
        self.assertEqual(candidate_output_code(255), 0)
        self.assertEqual(candidate_output_code(-256), 511)
        self.assertEqual(signed_measurement(255), (255, False))
        self.assertEqual(signed_measurement(-256), (-255, False))

    def test_out_of_domain_values_are_rejected(self):
        for value in (-257, 256):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    candidate_output_code(value)

    def test_trace_requires_schema_and_channel_fields(self):
        import io
        header = {"schema": "ym2413-nuked-opll-trace-v1", "time_unit": "opll_clock_call"}
        row = {"call_index": 0, "cycle_before": 0, "op_phase_index": 0,
               "op_output": 8, "ch_out": 1}
        _, rows = iter_trace(io.StringIO(json.dumps(header) + "\n" + json.dumps(row) + "\n"))
        self.assertEqual(len(rows), 1)
        with self.assertRaises(ValueError):
            iter_trace(io.StringIO(json.dumps({"schema": "wrong"}) + "\n"))

    def test_published_prediction_artifact_requires_1024_by_16(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "predicted-sine.txt"
            path.write_text((" ".join(["0"] * 16) + "\n") * 1024, encoding="ascii")
            self.assertEqual(len(load_predicted_sine(path)), 1024)
            path.write_text("0 1\n", encoding="ascii")
            with self.assertRaises(ValueError):
                load_predicted_sine(path)

    def test_characterization_reports_prediction_without_claiming_dac_word(self):
        rows = [{"call_index": 0, "cycle_before": 0, "op_phase_index": 0,
                 "op_output": 8, "ch_out": 1}]
        predicted = [[1] + [0] * 15] + [[0] * 16 for _ in range(1023)]
        report = characterize(rows, predicted, 0)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["records"][0]["candidate_output_code"], 254)
        self.assertEqual(report["published_prediction"]["exact_matches"], 1)
        self.assertIn("not a confirmed physical DAC", report["evidence_boundary"])


if __name__ == "__main__":
    unittest.main()
