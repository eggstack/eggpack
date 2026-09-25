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

Local payload (`prepare_staging_payload`) revalidates the finalized root against the contract and manifest, rejects missing/extra/tampered files, writes a standalone `release-manifest.json` that decodes back to the exact M004 manifest, generates deterministic `install.sh`/`install.ps1` with exact-tag origins (`https://github.com/<owner>/<repo>/releases/download/<tag>`), and emits a deterministic `StagingPayloadV1`. M004 root semantics are unchanged.

See `plans/implementation/ci-release-orchestration/003a-local-staging-payload-and-github-draft-adapter.md` and `plans/closure/ci-release-orchestration/003a-status.md`.
