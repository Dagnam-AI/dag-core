"""Run against the BUILT wheel (scripts/check.sh installs it); never against the source tree."""

import dagnam_core
from dagnam_core import hygiene


def test_the_package_reports_its_version_and_api() -> None:
    assert dagnam_core.__version__ == "0.1.0.dev0"
    assert hygiene.CORE_API == 1
