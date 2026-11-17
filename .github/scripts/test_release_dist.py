"""Tests for release_dist.py: the file set a release may hold, and the TestPyPI comparison.

Stdlib only (no pytest), so CI runs it on a bare runner:

    python3 -m unittest discover -s .github/scripts
"""

import contextlib
import hashlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import release_dist

VERSION = "1.2.3"
# The concrete file name each artifact must hold, spelled out here and not derived from the
# script's table, so a wrong pattern in the script is a failing test and not a copy of itself.
PLATFORMS = {
    "linux-x86_64-gnu": "manylinux_2_17_x86_64.manylinux2014_x86_64",
    "linux-aarch64-gnu": "manylinux_2_17_aarch64.manylinux2014_aarch64",
    "linux-x86_64-musl": "musllinux_1_2_x86_64",
    "linux-aarch64-musl": "musllinux_1_2_aarch64",
    "macos-x86_64": "macosx_10_12_x86_64",
    "macos-aarch64": "macosx_11_0_arm64",
    "windows-x86_64": "win_amd64",
}
KINDS = {"abi3": "cp312-abi3", "ft": "cp314-cp314t"}


def artifacts() -> dict[str, str]:
    """Artifact name -> file name: 7 platforms x 2 kinds, and the sdist."""
    files = {
        f"dist-{platform}-{kind}": f"dagnam_core-{VERSION}-{tag}-{plat}.whl"
        for platform, plat in PLATFORMS.items()
        for kind, tag in KINDS.items()
    }
    files["dist-sdist"] = f"dagnam_core-{VERSION}.tar.gz"
    return files


class WorkDir(unittest.TestCase):
    """A scratch directory holding a pyproject.toml, which is the cwd the script expects."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        (self.root / "pyproject.toml").write_text(f'[project]\nversion = "{VERSION}"\n')
        cwd = os.getcwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, cwd)

    def stage(self, files: dict[str, str]) -> Path:
        stage = self.root / "stage"
        for artifact, name in files.items():
            (stage / artifact).mkdir(parents=True)
            (stage / artifact / name).write_bytes(name.encode())
        return stage

    def verify_fails(self, stage: Path) -> str:
        with self.assertRaises(SystemExit) as raised:
            release_dist.verify(stage, self.root / "out")
        return str(raised.exception.code)


class Verify(WorkDir):
    def test_there_are_15_artifacts(self) -> None:
        self.assertEqual(len(artifacts()), 15)
        self.assertEqual(sorted(release_dist.expected(VERSION)), sorted(artifacts()))

    def test_exactly_the_expected_artifacts_pass_and_are_copied_out(self) -> None:
        stage = self.stage(artifacts())
        with contextlib.redirect_stdout(io.StringIO()) as out:
            release_dist.verify(stage, self.root / "out")
        self.assertEqual(out.getvalue().strip(), "15 artifacts, 15 files, all expected")
        self.assertEqual(
            sorted(p.name for p in (self.root / "out").iterdir()), sorted(artifacts().values())
        )

    def test_the_wheel_without_the_legacy_alias_spelling_also_passes(self) -> None:
        files = artifacts()
        files["dist-linux-x86_64-gnu-abi3"] = (
            f"dagnam_core-{VERSION}-cp312-abi3-manylinux_2_17_x86_64.whl"
        )
        with contextlib.redirect_stdout(io.StringIO()):
            release_dist.verify(self.stage(files), self.root / "out")

    def test_14_artifacts_fail_naming_the_missing_one(self) -> None:
        files = artifacts()
        del files["dist-windows-x86_64-ft"]
        self.assertIn("missing ['dist-windows-x86_64-ft']", self.verify_fails(self.stage(files)))

    def test_16_artifacts_fail_naming_the_extra_one(self) -> None:
        files = artifacts()
        files["dist-linux-riscv64-gnu-abi3"] = f"dagnam_core-{VERSION}-cp312-abi3-linux_riscv64.whl"
        self.assertIn(
            "unexpected ['dist-linux-riscv64-gnu-abi3']", self.verify_fails(self.stage(files))
        )

    def test_the_old_manylinux_2_28_wheel_is_refused(self) -> None:
        files = artifacts()
        files["dist-linux-x86_64-gnu-abi3"] = (
            f"dagnam_core-{VERSION}-cp312-abi3-manylinux_2_28_x86_64.whl"
        )
        self.assertIn("dist-linux-x86_64-gnu-abi3 must hold", self.verify_fails(self.stage(files)))

    def test_a_free_threaded_wheel_in_the_abi3_artifact_is_refused(self) -> None:
        files = artifacts()
        # the 3.14t wheel uploaded under the abi3 artifact's name
        files["dist-macos-aarch64-abi3"] = files["dist-macos-aarch64-ft"]
        self.assertIn("dist-macos-aarch64-abi3 must hold", self.verify_fails(self.stage(files)))

    def test_a_wheel_of_another_version_is_refused(self) -> None:
        files = artifacts()
        files["dist-sdist"] = "dagnam_core-9.9.9.tar.gz"
        self.assertIn("dist-sdist must hold", self.verify_fails(self.stage(files)))

    def test_an_artifact_holding_two_files_is_refused(self) -> None:
        stage = self.stage(artifacts())
        (stage / "dist-sdist" / "stray.txt").write_text("x")
        self.assertIn("dist-sdist must hold", self.verify_fails(stage))

    def test_an_empty_artifact_is_refused(self) -> None:
        stage = self.stage(artifacts())
        (stage / "dist-sdist" / f"dagnam_core-{VERSION}.tar.gz").unlink()
        self.assertIn("dist-sdist must hold", self.verify_fails(stage))


class Compare(WorkDir):
    def setUp(self) -> None:
        super().setUp()
        self.dist = self.root / "dist"
        self.dist.mkdir()
        (self.dist / "a.whl").write_bytes(b"a")

    @staticmethod
    def served(files: dict[str, bytes]) -> mock.MagicMock:
        body = {
            "urls": [
                {"filename": n, "digests": {"sha256": hashlib.sha256(b).hexdigest()}}
                for n, b in files.items()
            ]
        }
        response = mock.MagicMock()
        response.__enter__.return_value = io.BytesIO(json.dumps(body).encode())
        return mock.MagicMock(return_value=response)

    def compare(self, files: dict[str, bytes]) -> str:
        with (
            mock.patch.object(release_dist.urllib.request, "urlopen", self.served(files)),
            mock.patch.object(release_dist.time, "sleep"),
            contextlib.redirect_stdout(io.StringIO()) as out,
        ):
            release_dist.compare(self.dist)
        return out.getvalue()

    def test_the_same_files_pass(self) -> None:
        self.assertIn("exactly this run's 1 files", self.compare({"a.whl": b"a"}))

    def test_other_bytes_under_the_same_name_fail(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            self.compare({"a.whl": b"other"})
        self.assertIn("different bytes", str(raised.exception.code))

    def test_an_extra_file_on_testpypi_fails(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            self.compare({"a.whl": b"a", "b.whl": b"b"})
        self.assertIn("['b.whl']", str(raised.exception.code))

    def test_an_empty_dist_fails(self) -> None:
        (self.dist / "a.whl").unlink()
        with self.assertRaises(SystemExit):
            release_dist.compare(self.dist)


if __name__ == "__main__":
    unittest.main()
