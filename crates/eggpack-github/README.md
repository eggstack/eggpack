# eggpack-github

GitHub draft release staging adapter and local staging payload materializer (CI M003a).

- Draft only; exact existing tag only; no publication.
- Eggpack never creates, moves, force-updates, or deletes tags; annotated tags are peeled with a bounded loop to an exact commit.
- Draft creation sets `draft: true` and `make_latest: false`; there is no API/CLI path that publishes a release.
- Published or immutable releases are never mutated; unexpected or mismatched assets fail closed without clobber.
- Same-name assets are reused only on exact size plus `sha256:<digest>` match.
- A zero-byte `starter` asset left by a failed 502 upload is the only narrowly recoverable deletion; the invocation still fails and a later explicit rerun may resume.
- Credentials are environment-only (`GITHUB_TOKEN`); tokens never appear in argv, config, evidence, errors, or `Debug` output.
- Production endpoints are fixed to `https://api.github.com` with uploads constrained to `https://uploads.github.com`; every request uses `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2026-03-10`, and a bounded `User-Agent`.
- HTTP retries and redirects are disabled via the lean `eggfetch-core` profile (`standard-http1`, `tls-rustls`, `tls-native-roots`, `json`); 3xx fails closed.
- Bounded timeouts and metadata body sizes apply to every request; local staging paths reject symlinks and traversal.
- Incomplete drafts remain visible for maintainer inspection after failure; reruns reconcile exact state without automatic retry or broad cleanup.
- Production asset files are opened once, length- and SHA-256-verified in bounded 64 KiB chunks, then uploaded from that same handle as a known-length one-shot stream. No full-file upload buffer is materialized.
- Asset reconciliation reads every page under the policy bound (default 16, maximum 32); a full final allowed page fails closed because completeness is unknown.
- Upload templates are parsed and checked against the exact HTTPS `uploads.github.com` origin and current release path. Asset names are encoded as URL query pairs and round-trip unchanged.

Local payload (`prepare_staging_payload`) revalidates the finalized root against the contract and manifest, rejects missing/extra/tampered files, writes a standalone `release-manifest.json` that decodes back to the exact M004 manifest, generates deterministic `install.sh`/`install.ps1` with exact-tag origins (`https://github.com/<owner>/<repo>/releases/download/<tag>`), and emits a deterministic `StagingPayloadV1`. M004 root semantics are unchanged.

See `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md` and `plans/closure/ci-release-orchestration/003a-status.md`.

M003b consumes this adapter from one generated `stage` job only: the workflow
checks out the exact tag, downloads the exact aggregate artifact, runs the
payload materializer, then reconciles the draft with environment-only
credentials, and uploads the bounded receipt. No other job gains write
authority; reruns reuse exact drafts/assets without clobber. Live draft
evidence is tracked by `plans/closure/ci-release-orchestration/003b-status.md`.
