# eggpack-ci

Provider-neutral projection of a resolved `ReleasePlan` plus `BuildBindingsV1` into a bounded `CIPlan`, with deterministic checked-in GitHub Actions rendering and drift checking. Builder arguments are derived through `eggpack_core::cargo_command`; this crate does not infer package/bin identities, execute builds, perform qualification, create final manifests, stage releases, publish, access GitHub, or accept arbitrary YAML/step input.

The renderer requires caller-supplied runner labels and full immutable action commit pins. Runner policy declares whether cross-build images already contain cargo-zigbuild and Zig; M001 does not auto-install either tool. Generated build jobs use read-only permissions and hand off private candidate binaries as internal workflow artifacts. Qualification remains unresolved intent and no generated workflow claims release completion.

`render_github` is pure. `check_github` compares deterministic bytes after CRLF-to-LF normalization and reports drift without changing files.

Runner labels and action commit pins are policy inputs rather than CIPlan data.
Cross-build runners must declare preinstalled cargo-zigbuild and Zig; generated
jobs check the configured cargo-zigbuild version and never install tools.
