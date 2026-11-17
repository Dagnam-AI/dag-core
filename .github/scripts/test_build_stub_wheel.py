"""Tests for build_stub_wheel.py: the pure-Python stub wheel is valid, inert and reproducible.

Stdlib only (no pytest, no pip), so CI runs it on a bare runner:

    python3 -m unittest discover -s .github/scripts

The real install through pip and uv, next to the native wheels, is the `select` job of
release-build.yml.
"""

import base64
import csv
import hashlib
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

import build_stub_wheel

VERSION = "1.2.3"
PYPROJECT = f"""\
[project]
name = "dagnam-core"
version = "{VERSION}"
description = "Compiled core."
requires-python = ">=3.12"
license = "Apache-2.0"
classifiers = ["Programming Language :: Rust", "Typing :: Typed"]
"""
DIST_INFO = f"dagnam_core-{VERSION}.dist-info"


class StubWheel(unittest.TestCase):
    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        (self.root / "pyproject.toml").write_text(PYPROJECT)
        (self.root / "LICENSE").write_text("the licence text\n")
        cwd = os.getcwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, cwd)
        self.wheel = build_stub_wheel.build(self.root / "out")

    def read(self, name: str) -> str:
        with zipfile.ZipFile(self.wheel) as z:
            return z.read(name).decode()

    def test_the_file_is_the_universal_wheel_of_the_pyproject_version(self) -> None:
        self.assertEqual(self.wheel.name, f"dagnam_core-{VERSION}-py3-none-any.whl")

    def test_it_holds_the_stub_the_dist_info_and_the_licence_and_no_compiled_module(self) -> None:
        with zipfile.ZipFile(self.wheel) as z:
            self.assertEqual(
                z.namelist(),
                [
                    "dagnam_core/__init__.py",
                    "dagnam_core/hygiene.py",
                    f"{DIST_INFO}/METADATA",
                    f"{DIST_INFO}/WHEEL",
                    f"{DIST_INFO}/licenses/LICENSE",
                    f"{DIST_INFO}/RECORD",
                ],
            )
        self.assertEqual(self.read(f"{DIST_INFO}/licenses/LICENSE"), "the licence text\n")

    def test_the_wheel_file_says_pure_python_and_any_platform(self) -> None:
        lines = self.read(f"{DIST_INFO}/WHEEL").splitlines()
        self.assertIn("Root-Is-Purelib: true", lines)
        self.assertIn("Tag: py3-none-any", lines)
        self.assertIn("Wheel-Version: 1.0", lines)

    def test_the_metadata_names_what_the_real_wheels_name(self) -> None:
        head = self.read(f"{DIST_INFO}/METADATA").split("\n\n", 1)[0].splitlines()
        for line in (
            "Metadata-Version: 2.4",
            "Name: dagnam-core",
            f"Version: {VERSION}",
            "Summary: Compiled core.",
            "License-Expression: Apache-2.0",
            "License-File: LICENSE",
            "Requires-Python: >=3.12",
            "Classifier: Programming Language :: Rust",
            "Classifier: Typing :: Typed",
        ):
            self.assertIn(line, head)

    def test_record_lists_every_file_with_its_hash_and_size(self) -> None:
        with zipfile.ZipFile(self.wheel) as z:
            rows = list(csv.reader(z.read(f"{DIST_INFO}/RECORD").decode().splitlines()))
            self.assertEqual([r[0] for r in rows], z.namelist())
            for name, digest, size in rows[:-1]:
                data = z.read(name)
                raw = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=")
                self.assertEqual((digest, size), (f"sha256={raw.decode()}", str(len(data))))
        self.assertEqual(rows[-1], [f"{DIST_INFO}/RECORD", "", ""])

    def test_building_twice_gives_the_same_bytes(self) -> None:
        again = build_stub_wheel.build(self.root / "again")
        self.assertEqual(again.read_bytes(), self.wheel.read_bytes())

    def test_the_entries_carry_the_fixed_timestamp_and_plain_permissions(self) -> None:
        with zipfile.ZipFile(self.wheel) as z:
            for entry in z.infolist():
                self.assertEqual(entry.date_time, (1980, 1, 1, 0, 0, 0))
                self.assertEqual(entry.external_attr >> 16, 0o644)

    def run_python(self, code: str) -> subprocess.CompletedProcess[str]:
        """Python with the unpacked wheel first on sys.path (what an install puts there)."""
        site = self.root / "site"
        with zipfile.ZipFile(self.wheel) as z:
            z.extractall(site)
        source = f"import sys; sys.path.insert(0, {str(site)!r})\n{code}"
        return subprocess.run(
            [sys.executable, "-S", "-c", source], capture_output=True, text=True, cwd=self.root
        )

    def test_the_package_imports_and_reports_the_version_like_the_real_one(self) -> None:
        done = self.run_python("import dagnam_core; print(dagnam_core.__version__)")
        self.assertEqual((done.returncode, done.stdout.strip()), (0, VERSION), done.stderr)

    def test_importing_hygiene_raises_the_import_error_callers_catch(self) -> None:
        code = (
            "try:\n"
            "    from dagnam_core import hygiene\n"
            "except ImportError as error:\n"
            "    print(error)\n"
            "else:\n"
            "    raise SystemExit('imported')\n"
        )
        done = self.run_python(code)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(
            done.stdout.strip(),
            "dagnam-core has no compiled build for this interpreter or platform; "
            "callers fall back to their pure-Python code",
        )

    def test_the_stub_has_no_native_module(self) -> None:
        done = self.run_python("import dagnam_core\nimport dagnam_core._native")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("ModuleNotFoundError", done.stderr)


if __name__ == "__main__":
    unittest.main()
