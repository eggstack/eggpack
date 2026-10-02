# Quickstart: resolve a release and render its workflow

This guide resolves a one-target Linux release and renders its GitHub Actions
workflow using the `eggpack` CLI. Every command below was run verbatim against
the workspace and produces the quoted output. No network access, no real
build, no GitHub token: resolution and rendering are pure file-in/file-out.

Prerequisites: Rust stable (MSRV 1.89), git, and a checkout of this repo.

Build the CLI once:

```sh
cargo build -p eggpack-cli
EGGPACK=./target/debug/eggpack
$EGGPACK --version   # eggpack 0.1.0
```

## 1. Create a scratch release repo

The resolver binds the release to an exact git commit: `HEAD` under
`--source-root` must equal `--source-revision`, else it fails closed.

```sh
mkdir -p eggdemo/out && cd eggdemo
git init -q && git config user.email doc@example.com && git config user.name doc
```

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

Commit, then record `HEAD` — it becomes the release's source revision:

```sh
git add . && git commit -qm demo
REV=$(git rev-parse HEAD)   # 40 lowercase hex chars
```

## 2. Resolve the release plan

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
(draft identity). A wrong `--source-revision` or uncommitted `HEAD` fails
closed here.

## 3. Render the workflow

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

## 4. Check for drift

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

Any manual edit is caught — append a line and re-run to see:

```text
eggpack: ci check: drift detected (expected 6831 bytes, found 6845 bytes, first difference at Some(6831))
```

(exit 1). Comparison is CRLF→LF only; there is no fuzzy matching.

## Next steps

- Full command reference: [../crates/eggpack-cli/README.md](../crates/eggpack-cli/README.md).
- What each stage means: [../architecture/overview.md](../architecture/overview.md)
  (pipeline), [../architecture/core.md](../architecture/core.md),
  [../architecture/ci.md](../architecture/ci.md).
- After planning come build → qualify → finalize → stage; the generated
  workflow replays exactly these steps in CI.
