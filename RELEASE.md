# Releasing dagnam-core

A release is the git tag `core/vX.Y.Z`. **Pushing the tag, and approving the publish, are the
maintainer's own acts**: nothing in this repository tags or publishes by itself, and a task, a
review or a merge never authorises one.

`.github/workflows/release.yml` does the rest, and **only a tag push can check, attest or
publish** (a manual run, even of a tag, cannot). A pull request that changes any release input
(the workflow, the reusable build workflow `release-build.yml`, the script or pinned requirements,
the toolchain, `Cargo.lock`, `Cargo.toml` or `pyproject.toml`), or a manual run, builds and tests
everything and publishes nothing. `release.yml` holds the check, collect, attest and publish jobs
(PyPI's trusted publisher is bound to that file name); the build and test jobs live in
`release-build.yml` only to keep both files under the 500-line cap. On a tag it checks that the ref
is a tag, that the tagged commit is on `main`, that the tag, `pyproject.toml` and `CHANGELOG.md`
name one version, and that CI is green on that commit; builds every wheel (two per platform:
`abi3` and free-threaded 3.14t) and the sdist; installs each wheel into a clean environment
on its own platform and runs `tests/` (the glibc wheels also inside the manylinux2014 image, glibc
2.17); checks the release is exactly the 15 expected files; attests them;
publishes to TestPyPI, checks TestPyPI serves the same bytes, installs from there and runs it; and
only then publishes to PyPI through the `pypi` environment (trusted publishing, no token secrets).
PyPI takes no version back, so every step before the last is there to stop a bad one.

## One-time setup

- **Trusted publishers.** PyPI and TestPyPI each have a publisher for the project `dagnam-core`:
  owner `Dagnam-AI`, repository `dag-core`, workflow `release.yml`, environment `pypi` on PyPI and
  `testpypi` on TestPyPI. Until the first release these are only *pending* publishers: the name is
  not registered, a pending publisher does not reserve it, and the first publish creates the
  project. Cut `0.1.0` soon after registering; a squatter who takes the name first makes the
  upload fail (closed), and the release waits for a different name.
- **Environments restricted to tags.** Both environments (`pypi`, `testpypi`) deploy only from tags
  `core/v*`, so that a branch with a copy of the workflow cannot use them. Neither has a required
  reviewer: the tag is the release act and the maintainer who pushes it is the approval.

  ```sh
  for env in pypi testpypi; do
    gh api -X PUT repos/Dagnam-AI/dag-core/environments/$env --input - <<'JSON'
  {"deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}
  JSON
    gh api -X POST "repos/Dagnam-AI/dag-core/environments/$env/deployment-branch-policies" \
      -f name='core/v*' -f type=tag
  done
  ```

  Check: `gh api repos/Dagnam-AI/dag-core/environments/pypi/deployment-branch-policies` lists one
  policy of type `tag`.
- **A tag ruleset** named `release tags` on `refs/tags/core/v*` with the rules `deletion`, `non_fast_forward` and `update`, so a published tag can never be moved or removed.

## Steps

1. **Rehearse (mandatory before the first tag, and again whenever `release.yml`,
   `rust-toolchain.toml` or the PyO3 or maturin versions change).** Open a pull request that
   touches one of those inputs (it runs by itself), or run the workflow from `main`
   with Actions, "Release dagnam-core to PyPI", Run workflow. Every build, test, sdist and collect
   job must be green; the check, attest and publish jobs are skipped on purpose. Do not tag on a
   red or missing rehearsal.
2. Pick the version `X.Y.Z`. On a branch, set it in `pyproject.toml` (the crates' own versions
   are not published and do not need to follow), change the version in `tests/test_hygiene.py`,
   and add a `## X.Y.Z` section to `CHANGELOG.md`. Merge it to `main` by pull request.
3. Wait for CI on that `main` commit to finish green: the release refuses a commit whose `gate`
   job is not. A tag pushed too early fails the `check` job with `in_progress`; re-run the workflow
   once CI is green.
4. Tag the merged commit and push the tag:
   `git tag -a core/vX.Y.Z <sha> -m "dagnam-core X.Y.Z" && git push origin core/vX.Y.Z`.
5. The `pypi` job runs on its own once the TestPyPI smoke test passes; watch it.

## When a run fails

- **Before `publish-testpypi` ran**, nothing was published. A transient cause (CI still running,
  a runner or network flake) is fixed by re-running the failed jobs. If the cause is a defect in
  the code or the workflow, a re-run uses the same tagged files and fails the same way: switch the
  `release tags` ruleset to disabled, delete the tag, switch it back, fix on `main`, rehearse again and re-tag, as no file went
  to any index.
- **After files went to TestPyPI or PyPI**, do not move the tag. TestPyPI and PyPI skip files
  that are already there, so re-running a failed publish or smoke job finishes the upload (the
  smoke job fails if TestPyPI holds bytes that differ from this run's). **Re-run only the
  failed jobs, never all jobs**: a full re-run rebuilds the wheels, and rebuilt wheels differ in
  bytes (zip timestamps), so TestPyPI keeps the first bytes, the comparison fails on every later
  attempt, and the version then needs a patch release. A defect that needs a code
  or workflow change means a new patch version, because PyPI never lets a file name be reused.
- **A release that stops at TestPyPI** (a file TestPyPI or PyPI refuses, a smoke failure after the
  upload) leaves files on an index, so its tag is spent: **a tag cannot be reused**, and the fix is
  the next patch version, not a moved or re-pushed tag. `0.1.0` is the example: its sdist lacked
  the `LICENSE` its metadata names (maturin 1.15.0), the index refused it, nothing reached PyPI,
  and the fix shipped as `0.1.1` with a check in the sdist job. Land the fix on `main`, rehearse,
  bump the version and changelog, tag the new version.
- **TestPyPI is down.** `publish-pypi` needs the TestPyPI smoke test, so a release waits. Re-run
  the jobs once it is back; do not bypass the smoke test.

## Verify the release

From a neutral directory (not this checkout), in a fresh environment, with `X.Y.Z` the release:

```sh
cd "$(mktemp -d)"
uv venv --python 3.12 && . .venv/bin/activate
uv pip install --refresh dagnam-core==X.Y.Z
python -c "from dagnam_core import hygiene; print(hygiene.CORE_API)"   # prints 2
```

PyPI's index can lag an upload by a few minutes; retry the install with `--refresh` before
suspecting the release.

## Known accepted risks

Reviewed 2026-11-16; review again before 2027-05-16 or when the build changes. None of these
touches the sdist or wheel tests, which use hash-pinned requirements (`.github/ci-requirements.txt`
and `.github/ci-build-requirements.txt`).

- **maturin-action fetches tools it cannot verify.** It downloads the maturin binary named by
  `MATURIN_VERSION` from the GitHub release, without a checksum (the release's assets are not
  immutable); the version is pinned, the bytes are not. Inside a Linux container it also runs the
  rustup installer and uv's installer (when the image lacks them), `uv tool install patchelf`
  (unpinned; patchelf rewrites the shipped `.so`) and `pip install --user cffi`, and it forwards
  the run's `ACTIONS_*` variables into the container. The Linux wheels are therefore only as
  trustworthy as these tools. Mitigations: the images are pinned by digest, and `collect` accepts
  only the 15 expected files.
- **Container image digests are refreshed by hand.** The build and glibc-test images
  (`manylinux2014`, `manylinux2014-cross`, `rust-musl-cross`) are pinned to the digests in
  `release-build.yml`; they are re-resolved and re-rehearsed when the workflow changes, not on a
  schedule. `quay.io/pypa/manylinux2014_x86_64` and `_aarch64` are pinned from their `latest` tag.
  manylinux 2014 (CentOS 7) is end-of-life upstream but still built by pypa; if a refresh stops
  publishing it, the glibc floor moves to `manylinux_2_28` in the same change as a changelog line.
- **Toolchains come from their own servers:** rustup and the Rust toolchain from rust-lang.org (a
  checksum-verified channel manifest), uv and its pinned version from GitHub releases via
  setup-uv (which checks the version's known checksum), and the Python builds uv installs (3.12,
  3.14t) from python-build-standalone. Crates are fetched from crates.io under `Cargo.lock`.
- **The wheel and sdist tests run on shared GitHub runners,** which have passwordless sudo and the
  run's token in memory. A malicious test dependency is blocked by hash pinning; a malicious
  runner is not in scope.
