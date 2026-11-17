"""The uv version is written in three places of the release workflows: they must agree.

release.yml (the smoke job) and release-build.yml (every other job) each have a workflow-level
`UV_VERSION`, because `env` is not available to a job that calls a reusable workflow, and the
Alpine test image is tagged with the same version.
"""

import re
import unittest
from pathlib import Path

WORKFLOWS = Path(__file__).resolve().parents[1] / "workflows"


class UvVersion(unittest.TestCase):
    def read(self, name: str) -> str:
        return (WORKFLOWS / name).read_text()

    def test_the_two_workflows_and_the_alpine_image_name_one_version(self) -> None:
        pinned = {
            "release.yml": re.findall(r'^  UV_VERSION: "([\d.]+)"', self.read("release.yml"), re.M),
            "release-build.yml": re.findall(
                r'^  UV_VERSION: "([\d.]+)"', self.read("release-build.yml"), re.M
            ),
            "alpine image": re.findall(
                r"ghcr\.io/astral-sh/uv:([\d.]+)-alpine@sha256:", self.read("release-build.yml")
            ),
        }
        for where, found in pinned.items():
            self.assertEqual(len(found), 1, f"{where}: want exactly one uv version, got {found}")
        self.assertEqual(len({v[0] for v in pinned.values()}), 1, pinned)


if __name__ == "__main__":
    unittest.main()
