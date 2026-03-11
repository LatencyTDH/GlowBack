# Releasing GlowBack

GlowBack publishes GitHub Releases from a real CI artifact instead of rebuilding at
release time.

## What the build pipeline produces

The Rust build pipeline (`.github/workflows/rust.yml`) now uploads a release-ready
artifact named `glowback-engine-linux-x86_64` whenever a `main` branch build
succeeds from either:

- a normal `push` to `main`, or
- a manual `workflow_dispatch` run of `rust.yml` on `main`

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
- `notes` *(optional)* — custom release notes; if omitted, GitHub generates them

### Selection behavior

If you leave `run_id` empty, the workflow searches for the **latest successful
`rust.yml` run on `main`** from a `push` or manual build that contains the
selected artifact name.

If you provide `run_id`, the workflow uses that exact build — as long as it:

- ran on `main`
- came from `push` or `workflow_dispatch`
- still has the requested artifact available

The release workflow never rebuilds the binary. It reuses the artifact that CI
already produced.

## Recommended release process

1. Merge the changes you want onto `main`.
2. Wait for the Rust build pipeline to finish, or manually run `rust.yml` on
   `main` if you need a fresh artifact without a new commit.
3. Run **Manual Release** with a new semver tag such as `v0.2.0`.
4. Leave `run_id` blank to use the latest eligible build, or set it explicitly
   if you want to pin the release to a specific CI run.
5. After the workflow completes, verify the assets and notes on the GitHub
   Releases page.

## Operational notes

- Releases are tied to the commit SHA from the selected CI run.
- Existing release tags are rejected to avoid accidental overwrites.
- Build artifacts are retained for 30 days by GitHub Actions.
- If the artifact you want has expired, rerun `rust.yml` on `main` and then cut
  the release from the fresh run.
