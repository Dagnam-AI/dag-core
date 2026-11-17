# Releasing dagnam-core

A release is the git tag `core/vX.Y.Z`. **Pushing the tag, and approving the publish, are the
maintainer's own acts**: nothing in this repository tags or publishes by itself, and a task, a
review or a merge never authorises one.

`.github/workflows/release.yml` does the rest. On a tag it checks the tagged commit is on `main`,
that the tag, `pyproject.toml` and `CHANGELOG.md` name one version, and that CI is green on that
commit; builds every wheel and the sdist; installs each wheel into a clean environment on its own
platform and runs `tests/`; attests the files; publishes to TestPyPI, installs from there and runs
it; and only then publishes to PyPI through the `pypi` environment (trusted publishing, no token
secrets). PyPI takes no version back, so every step before the last is there to stop a bad one.

## One-time setup

- PyPI and TestPyPI each have a trusted publisher for the project `dagnam-core`: owner
  `Dagnam-AI`, repository `dag-core`, workflow `release.yml`, environment `pypi` on PyPI and
  `testpypi` on TestPyPI.
- The repository has the two environments `testpypi` and `pypi`; `pypi` requires a reviewer.
- A tag ruleset allows only maintainers to create `core/v*` tags.

## Steps

1. Pick the version `X.Y.Z`. On a branch, set it in `pyproject.toml` (the crates' own versions
   are not published and do not need to follow), change the version in
   `tests/test_hygiene.py`, and add a `## X.Y.Z` section to `CHANGELOG.md`. Merge it to `main`
   by pull request.
2. Wait for CI on that `main` commit to finish green: the release refuses a commit whose `gate`
   job is not.
3. Optionally rehearse: Actions, "Release dagnam-core to PyPI", Run workflow on the branch with
   `publish` left unticked. It builds and tests every wheel and the sdist and publishes nothing.
4. Tag the merged commit and push the tag:
   `git tag -a core/vX.Y.Z <sha> -m "dagnam-core X.Y.Z" && git push origin core/vX.Y.Z`.
5. Approve the `pypi` environment when the run reaches it.
6. If a run fails after the tag exists, fix the cause and re-run the failed jobs. If the tag was
   wrong, do not move it: PyPI may already hold files. Bump the patch version instead.

## Verify the release

From a neutral directory (not this checkout), in a fresh environment:

```sh
cd "$(mktemp -d)"
uv venv && . .venv/bin/activate
uv pip install --refresh dagnam-core==0.1.0
python -c "from dagnam_core import hygiene; print(hygiene.CORE_API)"   # prints 1
```

PyPI's index can lag an upload by a few minutes; retry the install with `--refresh` before
suspecting the release.
