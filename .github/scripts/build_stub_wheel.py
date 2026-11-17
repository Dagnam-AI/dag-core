"""Build the pure-Python stub wheel of dagnam-core, with the standard library only.

    build_stub_wheel.py OUT_DIR   writes OUT_DIR/dagnam_core-<version>-py3-none-any.whl

Run from the repository root; the metadata comes from pyproject.toml and LICENSE.

Why a stub: pip and uv prefer a wheel to an sdist of the same version, and a specific wheel (a
native one) to a `py3-none-any` one. So every release also carries this wheel: a platform or
interpreter that has a native wheel gets it, and every other one (free-threaded 3.13, PyPy,
Windows arm64, an old glibc, ...) installs this stub and never falls back to a Rust build of the
sdist. The stub imports as unavailable: `dagnam_core.hygiene` raises ImportError, which the
callers (dagnam-contracts) catch before they run their pure-Python code.

The bytes are reproducible: a fixed timestamp, a fixed file order, no compression (a deflate
stream differs between zlib builds, a stored one does not).
"""

import base64
import hashlib
import sys
import tomllib
import zipfile
from pathlib import Path

UNAVAILABLE = (
    "dagnam-core has no compiled build for this interpreter or platform; "
    "callers fall back to their pure-Python code"
)
# The real package's __init__ exposes the version and nothing else; the stub mirrors it.
INIT = """\
\"\"\"Compiled core for the Dagnam.AI libraries (stub: no compiled build here).\"\"\"

from importlib.metadata import version

__version__ = version("dagnam-core")
"""
HYGIENE = f"""\
\"\"\"Data-hygiene bindings: not available in this stub.\"\"\"

raise ImportError(
    "{UNAVAILABLE}"
)
"""
DESCRIPTION = """\
This is the stub of dagnam-core, installed where no compiled wheel exists for the interpreter
or platform (free-threaded CPython 3.13, PyPy, Windows arm64, a glibc older than 2.17, ...).
It holds no compiled code: `import dagnam_core.hygiene` raises ImportError, and the packages
that depend on dagnam-core catch it and use their pure-Python implementation.
"""
# Wheels carry timestamps from 1980 on; maturin writes this one in the native wheels too.
TIMESTAMP = (1980, 1, 1, 0, 0, 0)


def record_hash(data: bytes) -> str:
    digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=")
    return f"sha256={digest.decode()}"


def build(out_dir: Path) -> Path:
    with open("pyproject.toml", "rb") as f:
        project = tomllib.load(f)["project"]
    ver = str(project["version"])
    dist_info = f"dagnam_core-{ver}.dist-info"
    metadata = [
        "Metadata-Version: 2.4",
        f"Name: {project['name']}",
        f"Version: {ver}",
        *(f"Classifier: {c}" for c in project["classifiers"]),
        "License-File: LICENSE",
        f"Summary: {project['description']}",
        f"License-Expression: {project['license']}",
        f"Requires-Python: {project['requires-python']}",
        "Description-Content-Type: text/plain",
    ]
    wheel = [
        "Wheel-Version: 1.0",
        "Generator: dagnam-core build_stub_wheel.py",
        "Root-Is-Purelib: true",
        "Tag: py3-none-any",
    ]
    files = {
        "dagnam_core/__init__.py": INIT.encode(),
        "dagnam_core/hygiene.py": HYGIENE.encode(),
        f"{dist_info}/METADATA": ("\n".join(metadata) + "\n\n" + DESCRIPTION).encode(),
        f"{dist_info}/WHEEL": ("\n".join(wheel) + "\n").encode(),
        f"{dist_info}/licenses/LICENSE": Path("LICENSE").read_bytes(),
    }
    record = "".join(f"{n},{record_hash(d)},{len(d)}\n" for n, d in files.items())
    files[f"{dist_info}/RECORD"] = (record + f"{dist_info}/RECORD,,\n").encode()

    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / f"dagnam_core-{ver}-py3-none-any.whl"
    with zipfile.ZipFile(path, "w", zipfile.ZIP_STORED) as wheel_zip:
        for name, data in files.items():
            entry = zipfile.ZipInfo(name, TIMESTAMP)
            entry.external_attr = 0o644 << 16
            wheel_zip.writestr(entry, data)
    return path


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    print(build(Path(sys.argv[1])))
