# Releasing GlowBack

GlowBack publishes GitHub Releases from a real CI artifact instead of rebuilding at
release time.

## What the build pipeline produces

The Rust build pipeline (`.github/workflows/rust.yml`) uploads a release-ready
artifact named `glowback-engine-linux-x86_64` whenever an eligible `main` branch
build succeeds from either:

- a normal `push` to `main`, or
- a manual `workflow_dispatch` run of `rust.yml` on `main`

Pull request builds are never eligible release sources.

That artifact contains:

- `glowback-engine-linux-x86_64.tar.gz`
- `glowback-engine-linux-x86_64.tar.gz.sha256`
- `glowback-engine-linux-x86_64.metadata.json`

The tarball includes the `gb-engine-service` release binary plus the license,
README, and build metadata.

## Manual release workflow

Use **Actions → Manual Release** (`.github/workflows/release.yml`) to publish a
versioned GitHub Release.

### Inputs

- `tag` **(required)** — the release tag to create, e.g. `v0.2.0`
- `release_name` *(optional)* — custom release title; defaults to `tag`
- `run_id` *(optional)* — exact `rust.yml` run ID to publish from
- `artifact_name` *(optional)* — artifact to attach; defaults to
  `glowback-engine-linux-x86_64`
- `prerelease` *(optional)* — mark the release as a prerelease
- `notes` *(optional)* — custom release notes; if omitted, the workflow asks the
  GitHub Releases API to generate notes anchored to the previous published
  release tag

### Selection behavior

If you leave `run_id` empty, the workflow now:

1. snapshots the current upstream `main` HEAD commit when the release is
   dispatched,
2. finds the matching upstream `rust.yml` run for that exact commit,
3. prefers the `push` build for that commit (falling back to a manual
   `workflow_dispatch` build only when needed), and
4. waits up to 30 minutes for that run to finish successfully and expose the
   selected artifact.

It does **not** fall back to an older successful run from a different commit, so
releases cannot silently publish stale assets.

If you provide `run_id`, the workflow uses that exact build — as long as it:

- belongs to the upstream repository running the release,
- is a `rust.yml` run,
- ran on `main`,
- came from `push` or `workflow_dispatch`, and
- still has the requested artifact available

The release workflow never rebuilds the binary. It reuses the artifact that CI
already produced.

### How automatic release-note deltas work

When `notes` is left blank, the workflow resolves the previous published release
from GitHub, sorted by version, and sends that tag to the
`releases/generate-notes` API as `previous_tag_name`.

That means:

- `v0.2.0` notes are generated from the delta since `v0.1.0`, not from the
  beginning of the repository, and
- the very first published release has no previous tag, so its generated notes
  legitimately cover the full history up to that release.

## Recommended release process

1. Merge the changes you want onto `main`.
2. Run **Manual Release** with a new semver tag such as `v0.2.0`.
3. Leave `run_id` blank to release from the exact `main` commit that was current
   when you started the workflow. If the Rust pipeline for that commit is still
   running, the release job waits for it automatically.
4. Use `run_id` only when you intentionally want to pin the release to a known
   successful upstream `main` build.
5. After the workflow completes, verify the assets and notes on the GitHub
   Releases page.

## Operational notes

- Releases are tied to the commit SHA from the selected CI run.
- Existing release tags are rejected to avoid accidental overwrites.
- Build artifacts are retained for 30 days by GitHub Actions.
- If the artifact you want has expired, rerun `rust.yml` on `main` and then cut
  the release from the fresh run.
- If the current `main` build fails, the release workflow fails too; fix the
  build first rather than releasing from a PR artifact or an older commit.
