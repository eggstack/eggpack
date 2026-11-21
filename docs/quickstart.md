# Quickstart: build a release locally and render its workflow

This walkthrough creates a sample project, builds and qualifies a Linux x86_64
release, then renders and checks a GitHub Actions workflow. The commands and
sample outputs are from an end-to-end run.

The local release path is `check` -> `plan` -> `package` -> `qualify` ->
`collect` -> `manifest`. It runs a real Cargo build. Both the CLI build and
release build may download uncached Cargo dependencies; the sample project has
none. The walkthrough does not use the GitHub API or require a GitHub token.
Workflow resolution checks the local Git checkout, while `ci generate` and
`ci check` operate on local files.

Prerequisites: Rust 1.89 or newer, Git, and the Rust toolchain named in
`pack-config.toml`. The package step invokes `cargo +<toolchain>`:

```sh
rustup toolchain install 1.89.0
```

**Host requirements.** The sample builds and natively qualifies a Linux x86_64
target, so use a matching Linux x86_64 host for the full local release path.
Planning and workflow generation are host-independent. Cross-host Cargo builds
may need a target linker, and native qualification fails if the host does not
match the target. These commands use a POSIX shell; on Windows, use Git Bash or
WSL for the host-independent steps.

Build the CLI once:

```sh
cargo build -p eggpack-cli
EGGPACK=./target/debug/eggpack
$EGGPACK --version   # eggpack 0.1.2
```

## 1. Create a scratch release repo

The later `ci _resolve-release` command verifies the local checkout against
`--source-revision` and checks that the tag peels to the same commit. Local
operator commands take `--source-revision` as an explicit value and do not
inspect Git.

Run the following setup from a directory outside the Eggpack checkout, such as
its parent directory. Cargo treats a package inside the Eggpack workspace as an
undeclared workspace member, which makes the sample build fail.

```sh
mkdir -p eggdemo/out && cd eggdemo
git init -q && git config user.email doc@example.com && git config user.name doc
```

Write these six input files. Validation rejects unknown fields and requires
`schema_version = 1`:

`contract.toml`: one direct-asset target plus a macOS spare (only the
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

`pack-config.toml`: producer policy for the Linux target (native Cargo
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

`build-bindings.toml`: logical-output → package/binary mapping (exact plan
coverage, no extras):

```toml
schema_version = 1

[targets]
"x86_64-unknown-linux-gnu" = [{ selector = { kind = "direct" }, package = "demo", binary = "demo" }]
```

`qualification-bindings.toml`: one smoke check for the executable target
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

`draft-template.json`: draft identity (tag is appended at resolve time, no
templating language):

```json
{"schema_version":1,"owner":"eggstack","repository":"demo","title_prefix":"demo "}
```

`github-policy.json`: caller-supplied runner labels + immutable action pins
(policy input, never plan data):

```json
{"preflight_runner":"ubuntu-latest","runners":[{"os":"linux","arch":"x86_64","label":"ubuntu-latest","cargo_zigbuild":false,"zig":false}],"checkout":{"reference":"actions/checkout@0123456789abcdef0123456789abcdef01234567"},"rust_toolchain":{"reference":"dtolnay/rust-toolchain@0123456789abcdef0123456789abcdef01234567"},"upload_artifact":{"reference":"actions/upload-artifact@0123456789abcdef0123456789abcdef01234567"},"download_artifact":{"reference":"actions/download-artifact@0123456789abcdef0123456789abcdef01234567"},"triggers":["push"],"timeout_minutes":60,"cancel_in_progress":true,"artifact_retention_days":7,"eggpack_tool":{"repo":"https://github.com/eggstack/eggpack","revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","package":"eggpack-cli","install_timeout_minutes":10},"release_inputs":{"contract":"contract.toml","release_plan":"out/release-plan.json","build_bindings":"build-bindings.toml","qualification_bindings":"qualification-bindings.toml","ci_plan":"out/release-ci-plan.json"}}
```

The action and Eggpack commit IDs above are placeholders with valid shapes, not
verified revisions. Replace them with approved immutable commits before running
the workflow. Then regenerate and recheck the workflow because the changed pins
change its bytes.

The sample project is a trivial Cargo binary with no dependencies, so its build
does not need to download crates:

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

Commit the sample and tag that commit. The revision recorded from `HEAD` and
the tag both identify this commit; `ci _resolve-release` verifies the match:

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

`--source-revision` must be 40 lowercase hexadecimal characters, and
`--release-id` must pass the tag-safety rules. `check` validates these values
without inspecting the checkout or running Git. The producer policy and both
binding documents accept TOML or JSON. The contract uses TOML and the release
plan uses JSON.

## 3. Resolve the release plan

`eggpack plan` takes the same inputs and writes the `ReleasePlan`: the
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

`--artifacts-dir` is the **per-target artifact root** written by `package`:
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

Keep `--output-dir` separate from `--artifacts-dir`; qualification must not
overwrite the candidate bytes it evaluates.

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

The parent of `--output-root` must already exist. This example creates
`out/release` first. The finalizer writes a new release root beneath it and does
not overwrite an existing root. `--artifacts-dir` points to the per-target
artifact root written by `qualify`.

`release-manifest.json` is written beside the finalized root, not inside it.

## 7. Derive the release manifest

`eggpack manifest` derives the release manifest independently, from the
finalized tree's bytes.

```sh
mkdir -p out/manifest
$EGGPACK manifest \
  --contract contract.toml \
  --release-plan out/release-plan.json \
  --build-bindings build-bindings.toml \
  --qualification-bindings qualification-bindings.toml \
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
writes a file. The bindings are required so the derived manifest projects the
same release graph `collect` projects.

The derived manifest must match the one beside the finalized root, if present.
For archive targets, `--artifacts-dir` is required so member bytes can be
compared with the candidates. The manifest records each artifact's name, size,
and SHA-256. These are integrity facts, not a signature or provenance claim.

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
A wrong `--source-revision` or a tag that peels to a different commit fails
closed. This checks committed `HEAD`, not uncommitted working-tree changes, so
commit all intended source before resolving the release.

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
No tag or revision is baked in: the same static shape serves future tags via
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

A mismatch exits with an error and is reported as drift. Comparison normalizes
CRLF to LF only; a reformatted but otherwise equivalent workflow still counts
as drift.

## Next steps

This example does not stage or publish a release. Draft staging is optional;
publication remains a separate human action.

- [CLI command reference](../crates/eggpack-cli/README.md): flags, outputs,
  and staging commands.
- [Architecture overview](../architecture/overview.md): pipeline and design
  boundaries.
- The generated workflow runs the configured build, qualification, gate, and
  aggregation jobs. An optional stage job can create a draft; it never publishes.
