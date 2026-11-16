import importlib.util
import sys
import unittest
from pathlib import Path
sys.path.insert(0, "/home/intpa/ssbu-mod-playbook/tools/eden-runner")
spec = importlib.util.spec_from_file_location("tile_proof", Path(__file__).with_name("steps.py"))
steps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(steps)


def lines(screen="css", targets=(1, 2, 3), hover="3"):
    result = [f"1 step screen={screen} dir=right from={i} to={j}" for i, j in enumerate(targets)]
    result.append(f"2 settled screen={screen} at=3 hovered={hover} x=0 y=0")
    return result


class RepeatEvidence(unittest.TestCase):
    def test_css_exact_steps_and_hover(self):
        self.assertEqual(steps.check_repeat(lines(), "css")["final_tile"], 3)

    def test_sss_does_not_invent_hover(self):
        self.assertEqual(len(steps.check_repeat(lines("sss", hover="none"), "sss")["steps"]), 3)

    def test_extra_repeat_fails(self):
        with self.assertRaises(AssertionError):
            steps.check_repeat(lines(targets=(1, 2, 3, 4)), "css")

    def test_missing_repeat_fails(self):
        with self.assertRaises(AssertionError):
            steps.check_repeat(lines(targets=(1, 2)), "css")

    def test_wrong_hover_fails(self):
        with self.assertRaises(AssertionError):
            steps.check_repeat(lines(hover="4"), "css")

if __name__ == "__main__":
    unittest.main()
