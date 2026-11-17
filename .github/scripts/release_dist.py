"""Release file checks for release.yml: what may be published, and TestPyPI parity.

    release_dist.py verify STAGE OUT   every artifact in STAGE is one of the 15 expected, with
                                       exactly the expected file; the files are copied to OUT
    release_dist.py compare DIST       TestPyPI serves exactly DIST's files, byte for byte

Run from the repository root; the version is the one in pyproject.toml.
"""

import hashlib
import json
import re
import shutil
import sys
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

# platform name (as in release.yml's matrix) -> the platform part of the wheel's tag
PLATFORMS = {
    "linux-x86_64-gnu": r"manylinux_2_28_x86_64",
    "linux-aarch64-gnu": r"manylinux_2_28_aarch64",
    "linux-x86_64-musl": r"musllinux_1_2_x86_64",
    "linux-aarch64-musl": r"musllinux_1_2_aarch64",
    "macos-x86_64": r"macosx_\d+_\d+_x86_64",
    "macos-aarch64": r"macosx_\d+_\d+_arm64",
    "windows-x86_64": r"win_amd64",
}
# build kind -> the interpreter part of the wheel's tag (the free-threaded wheel is not abi3)
KINDS = {"abi3": "cp312-abi3", "ft": "cp314-cp314t"}


def version() -> str:
    with open("pyproject.toml", "rb") as f:
        return str(tomllib.load(f)["project"]["version"])


def expected(ver: str) -> dict[str, re.Pattern[str]]:
    """Artifact name -> the one file name it may hold."""
    want = {
        f"dist-{platform}-{kind}": re.compile(rf"dagnam_core-{re.escape(ver)}-{tag}-{plat}\.whl")
        for platform, plat in PLATFORMS.items()
        for kind, tag in KINDS.items()
    }
    want["dist-sdist"] = re.compile(rf"dagnam_core-{re.escape(ver)}\.tar\.gz")
    return want


def verify(stage: Path, out: Path) -> None:
    want = expected(version())
    have = sorted(p.name for p in stage.iterdir())
    if have != sorted(want):
        sys.exit(
            f"artifacts differ from the allow-list: unexpected {sorted(set(have) - set(want))}, "
            f"missing {sorted(set(want) - set(have))}"
        )
    out.mkdir(parents=True)
    for name, pattern in want.items():
        files = list((stage / name).iterdir())
        if len(files) != 1 or not files[0].is_file() or not pattern.fullmatch(files[0].name):
            sys.exit(f"{name} must hold exactly one file matching {pattern.pattern}: {files}")
        shutil.copy2(files[0], out / files[0].name)
    print(f"{len(want)} artifacts, {len(want)} files, all expected")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compare(dist: Path) -> None:
    """Fail unless TestPyPI serves exactly these files, with these digests (waits for the index)."""
    local = {p.name: sha256(p) for p in dist.iterdir()}
    url = f"https://test.pypi.org/pypi/dagnam-core/{version()}/json"
    remote: dict[str, str] = {}
    for attempt in range(15):
        try:
            with urllib.request.urlopen(url, timeout=30) as response:
                remote = {
                    u["filename"]: u["digests"]["sha256"] for u in json.load(response)["urls"]
                }
        except (urllib.error.URLError, KeyError, json.JSONDecodeError):
            remote = {}
        if set(local) <= set(remote):
            break
        print(f"waiting for TestPyPI to list every file ({attempt + 1}/15)")
        time.sleep(60)
    if set(remote) != set(local):
        sys.exit(f"TestPyPI files differ from this run's: {sorted(set(remote) ^ set(local))}")
    wrong = sorted(n for n in local if remote[n] != local[n])
    if wrong:
        sys.exit(f"TestPyPI holds different bytes than this run built for: {wrong}")
    print(f"TestPyPI serves exactly this run's {len(local)} files")


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["verify", stage, out]:
            verify(Path(stage), Path(out))
        case ["compare", dist]:
            compare(Path(dist))
        case _:
            sys.exit(__doc__)
