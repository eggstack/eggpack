# Quickstart: build a release locally, and render its workflow

This guide builds a one-target Linux release end to end on a workstation with
the `eggpack` CLI, then renders the GitHub Actions workflow that replays the
same steps in CI. Every command below was run verbatim against the workspace
and produces the quoted output.

The first half is the **local producer operator path**: `check` → `plan` →
`package` → `qualify` → `collect` → `manifest`. It performs a real Cargo
build, but no network access and no GitHub token. The second half is the
**CI rendering path**: it is pure file-in/file-out.

Prerequisites: Rust (MSRV 1.89) and git, plus the Rust toolchain named in
`pack-config.toml` installed — `package` invokes `cargo +<toolchain>`:

```sh
rustup toolchain install 1.89.0
```

**Host scope of this walkthrough.** Steps 2–7 below build and qualify a
**Linux x86_64** release: `pack-config.toml` pins `host_os = "linux"` and
`host_arch = "x86_64"`, and a `native` qualification is host-matched proof, so
on a macOS or Windows host the `qualify` step records a host mismatch and
fails by design. Steps 1–3 and 8–9 are host-independent — the plan document and
the rendered workflow bytes are the same on every host. The commands below are
POSIX-shell commands; on Windows, run them from Git Bash or WSL.

Build the CLI once:

```sh
cargo build -p eggpack-cli
EGGPACK=./target/debug/eggpack
$EGGPACK --version   # eggpack 0.1.0
```

## 1. Create a scratch release repo

`ci _resolve-release` binds the release to an exact git commit: `HEAD` under
`--source-root` must equal `--source-revision`, and the tag must peel to the
same commit, else it fails closed. The local operator commands do not inspect a
checkout at all — they take `--source-revision` as an explicit value — but the
CI rendering path below does, so the scratch repo is a git repository.

```sh
mkdir -p eggdemo/out && cd eggdemo
git init -q && git config user.email doc@example.com && git config user.name doc
```

Create `eggdemo` **outside the Eggpack checkout** — for example as a sibling of
it, not inside it. Cargo treats a package directory inside the Eggpack workspace
as a workspace member that was never declared, and the `package` step below
then fails with `current package believes it's in a workspace when it's not`.
The scratch project is not part of Eggpack and must not be a member of its
workspace.

Write these five input files (exact bytes matter — validation is fail-closed
with `deny_unknown_fields` and `schema_version == 1` everywhere):

`contract.toml` — one direct-asset target plus a macOS spare (only the
selected target is planned):

```toml
schema_version = 1

[product]
id = "eggsact"
display_name = "eggsact"

[[targets]]
triple = "x86_64-unknown-linux-gnu"
aliases = ["linux-x64"]

[targets.asset]
kind = "direct"
asset = "{product}-{version}-{target}"
install = "{product}"

[targets.checksum]
sidecar = "{asset}.sha256"

[[targets]]
triple = "aarch64-apple-darwin"
aliases = ["macos-arm64"]

[targets.asset]
kind = "direct"
asset = "{product}-{version}-{target}"
install = "{product}"

[targets.checksum]
sidecar = "{asset}.sha256"
```

`pack-config.toml` — producer policy for the Linux target (native Cargo
build, native qualification, required tier):

```toml
schema_version = 1

[[targets]]
target = "x86_64-unknown-linux-gnu"
strategy = "native_cargo"
host_os = "linux"
host_arch = "x86_64"
toolchain = { rust = "1.89.0" }
floor = { kind = "none" }
qualification = "native"
support = "required"
```

`build-bindings.toml` — logical-output → package/binary mapping (exact plan
coverage, no extras):

```toml
schema_version = 1

[targets]
"x86_64-unknown-linux-gnu" = [{ selector = { kind = "direct" }, package = "demo", binary = "demo" }]
```

`qualification-bindings.toml` — one smoke check for the executable target
(required for `native` qualification; fixed argv, no shell):

```toml
schema_version = 1

[targets."x86_64-unknown-linux-gnu".smoke]
selector = { kind = "direct" }
argv = []
timeout_ms = 5000
stdout_limit = 1024
stderr_limit = 1024
```

`draft-template.json` — draft identity (tag is appended at resolve time, no
templating language):

```json
{"schema_version":1,"owner":"eggstack","repository":"demo","title_prefix":"demo "}
```

`github-policy.json` — caller-supplied runner labels + immutable action pins
(policy input, never plan data):

```json
{"preflight_runner":"ubuntu-latest","runners":[{"os":"linux","arch":"x86_64","label":"ubuntu-latest","cargo_zigbuild":false,"zig":false}],"checkout":{"reference":"actions/checkout@0123456789abcdef0123456789abcdef01234567"},"rust_toolchain":{"reference":"dtolnay/rust-toolchain@0123456789abcdef0123456789abcdef01234567"},"upload_artifact":{"reference":"actions/upload-artifact@0123456789abcdef0123456789abcdef01234567"},"download_artifact":{"reference":"actions/download-artifact@0123456789abcdef0123456789abcdef01234567"},"triggers":["push"],"timeout_minutes":60,"cancel_in_progress":true,"artifact_retention_days":7,"eggpack_tool":{"repo":"https://github.com/eggstack/eggpack","revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","package":"eggpack-cli","install_timeout_minutes":10},"release_inputs":{"contract":"contract.toml","release_plan":"out/release-plan.json","build_bindings":"build-bindings.toml","qualification_bindings":"qualification-bindings.toml","ci_plan":"out/release-ci-plan.json"}}
```

Finally the project `package` will build. It is a trivial Cargo binary with no
dependencies, so the build needs no network and no registry access:

```sh
mkdir -p demo/src
cat > demo/Cargo.toml <<'EOF'
[package]
name = "demo"
version = "0.1.0"
edition = "2021"
EOF
cat > demo/Cargo.lock <<'EOF'
version = 3

[[package]]
name = "demo"
version = "0.1.0"
EOF
printf 'fn main() {}\n' > demo/src/main.rs
```

Commit, then record `HEAD` — it becomes the release's source revision — and
tag it, because `ci _resolve-release` peels the tag:

```sh
git add . && git commit -qm demo
REV=$(git rev-parse HEAD)   # 40 lowercase hex chars
git tag 1.2.6
```

## 2. Validate the input set

`eggpack check` resolves the whole planning input set and writes nothing. It
reads the contract, the producer policy, and both bindings documents, and
fails closed if the policy does not resolve for the selected identity, if the
build bindings do not match the plan, or if the qualification bindings do not
match the plan and the build bindings.

```sh
$EGGPACK check \
  --contract contract.toml \
  --pack-config pack-config.toml \
  --build-bindings build-bindings.toml \
  --qualification-bindings qualification-bindings.toml \
  --selected linux-x64 \
  --release-id 1.2.6 \
  --source-revision "$REV"
```

Expected output:

```text
check: ok
```

`--source-revision` must be 40 lowercase hex characters and `--release-id` must
be exact-tag safe; both are checked. `check` does not inspect the checkout —
it never runs `git`.

## 3. Resolve the release plan

`eggpack plan` takes the same inputs and writes the `ReleasePlan` — the
release's *intent*, not evidence of anything built.

```sh
$EGGPACK plan \
  --contract contract.toml \
  --pack-config pack-config.toml \
  --build-bindings build-bindings.toml \
  --qualification-bindings qualification-bindings.toml \
  --selected linux-x64 \
  --release-id 1.2.6 \
  --source-revision "$REV" \
  --output out/release-plan.json
```

Expected output:

```text
plan: ok
```

`out/release-plan.json` is a schema-v1 JSON document carrying the
`release_id`, the `source_revision`, and one entry per planned target with its
policy, asset form, and floor status. Its bytes vary with the revision, so
the document is not quoted here.

## 4. Build the candidates

`eggpack package` performs a real Cargo build of every planned target and
stages the candidates under the output root.

```sh
$EGGPACK package \
  --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml \
  --source-root demo \
  --output-root out/build
```

Expected output:

```text
package: ok
```

The output root now holds one directory per target:

```text
$ ls out/build
x86_64-unknown-linux-gnu
$ ls out/build/x86_64-unknown-linux-gnu
build-handoff.json
candidates
$ ls out/build/x86_64-unknown-linux-gnu/candidates
candidate-direct
```

`build-handoff.json` is the canonical build descriptor (release identity, target,
strategy, and one entry per logical output with its size). `candidates/` holds
the candidate bytes under their canonical relative names. The build itself runs
in a private scratch directory, `out/build/.eggpack-build/`, which is recreated
on every run so a restarted stage starts from a clean Cargo target directory.

## 5. Qualify the candidates

`eggpack qualify` runs the configured qualification for every planned target
and writes the evidence.

```sh
$EGGPACK qualify \
  --contract contract.toml \
  --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml \
  --qualification-bindings qualification-bindings.toml \
  --artifacts-dir out/build \
  --output-dir out/qualification
```

Expected output:

```text
qualify: ok
```

`--artifacts-dir` is the **per-target artifact root** written by `package` —
the directory that holds `<target>/build-handoff.json` and `<target>/candidates/`,
not the `candidates/` directory itself. The output directory mirrors that
layout:

```text
$ ls out/qualification/x86_64-unknown-linux-gnu
build-handoff.json
candidates
evidence.json
```

`evidence.json` records the planned and actual classification, the actual host
the qualification ran on, the candidate digests and formats, and the bounded
process outcomes. This run used `qualification = "native"`, so the evidence
records the host it matched; a native run is host-matched proof and does not
prove a declared glibc or macOS floor.

## 6. Aggregate the finalized release tree

`eggpack collect` aggregates the qualified candidates into a finalized release
tree.

```sh
mkdir -p out/release
$EGGPACK collect \
  --contract contract.toml \
  --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml \
  --qualification-bindings qualification-bindings.toml \
  --artifacts-dir out/qualification \
  --output-root out/release/finalized
```

Expected output:

```text
collect: ok
```

```text
$ ls out/release
finalized
release-manifest.json
$ ls out/release/finalized
eggsact-1.2.6-x86_64-unknown-linux-gnu
eggsact-1.2.6-x86_64-unknown-linux-gnu.sha256
```

The parent of `--output-root` must already exist — `out/release` is created
here — because the finalizer writes one *new* release root under a
caller-secured parent and never overwrites an existing one. `--artifacts-dir`
is again the per-target artifact root, this time the one `qualify` wrote.

`release-manifest.json` lands **beside** the finalized root, never inside it.

## 7. Derive the release manifest

`eggpack manifest` derives the release manifest independently, from the
finalized tree's bytes.

```sh
mkdir -p out/manifest
$EGGPACK manifest \
  --contract contract.toml \
  --release-plan out/release-plan.json \
  --finalized-root out/release/finalized \
  --output out/manifest/release-manifest.json
```

Expected output:

```text
manifest: ok
```

The two manifests are the same document:

```text
$ diff out/release/release-manifest.json out/manifest/release-manifest.json
(identical)
```

`--output`'s parent directory must already exist, as for every command that
writes a file. `--artifacts-dir` is optional and is needed only for archive
targets, whose member digests are facts about the member bytes the finalized
tree does not retain.

The manifest carries the product, release, and source revision, and for each
target the asset form, the artifact name, its size, and its SHA-256. Those
are integrity facts about the bytes — not a signature and not a provenance
claim.

## 8. Render the workflow

The CI path resolves its own runtime identity, so it verifies the checkout and
the tag that the local operator commands take as explicit values.

```sh
$EGGPACK ci _resolve-release \
  --contract contract.toml \
  --pack-config pack-config.toml \
  --build-bindings build-bindings.toml \
  --qualification-bindings qualification-bindings.toml \
  --selected linux-x64 \
  --tag 1.2.6 \
  --source-revision "$REV" \
  --template draft-template.json \
  --source-root . \
  --output-plan out/release-plan.json \
  --output-ci-plan out/release-ci-plan.json \
  --output-github-policy out/draft-policy.json
```

Expected output:

```text
checked-out source matches release plan
resolved runtime release identity for tag 1.2.6
```

This writes `out/release-plan.json` (intent, not evidence),
`out/release-ci-plan.json` (executable graph), and `out/draft-policy.json`
(draft identity). The plan it writes is byte-identical to the one
`eggpack plan` wrote to the same path: both surfaces share one plan encoder.
A wrong `--source-revision`, an uncommitted `HEAD`, or a tag that does not peel
to the revision fails closed here.

Now render the workflow:

```sh
$EGGPACK ci generate \
  --ci-plan out/release-ci-plan.json \
  --github-policy github-policy.json \
  --output out/release.yml
```

Expected output:

```text
generated 6831 bytes to out/release.yml
```

The workflow is deterministic (same inputs → byte-identical bytes) and
read-only (`contents: read`): jobs `preflight` → `build_x86_64_unknown_linux_gnu`
→ `qualify_build_x86_64_unknown_linux_gnu` → `required_gate` → `aggregate`.
No tag or revision is baked in — the same static shape serves future tags via
the runtime `resolve` job (see `../crates/eggpack-ci/README.md`).

## 9. Check for drift

```sh
$EGGPACK ci check \
  --ci-plan out/release-ci-plan.json \
  --github-policy github-policy.json \
  --workflow out/release.yml
```

Expected output (exit 0, never writes):

```text
ci check: match (6831 bytes)
```

Any manual edit is caught. Append exactly this and re-run:

```sh
printf '# manual edit\n' >> out/release.yml
$EGGPACK ci check --ci-plan out/release-ci-plan.json \
  --github-policy github-policy.json --workflow out/release.yml
```

```text
eggpack: ci check: drift detected (expected 6831 bytes, found 6845 bytes, first difference at Some(6831))
```

(exit 1). Those byte counts are reproducible because the edit is exactly 14
bytes. Comparison is CRLF→LF only; there is no fuzzy matching, so a
reformatted-but-equivalent file is drift too.

## What this walkthrough does not do

No command above stages or publishes. Draft staging and publication remain
separate, explicitly authorized actions — see
[../crates/eggpack-cli/README.md](../crates/eggpack-cli/README.md) for the
staging runners and
[../architecture/cli.md](../architecture/cli.md) for the command taxonomy.

## Next steps

- Full command reference: [../crates/eggpack-cli/README.md](../crates/eggpack-cli/README.md).
- What each stage means: [../architecture/overview.md](../architecture/overview.md)
  (pipeline), [../architecture/core.md](../architecture/core.md),
  [../architecture/ci.md](../architecture/ci.md).
- The generated workflow replays exactly these steps in CI; after planning
  come build → qualify → finalize → stage.
