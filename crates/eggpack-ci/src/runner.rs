//! The provider-neutral runner command and its deterministic argument rendering.

use crate::policy::ReleaseIdentityMode;
use crate::render_github::shell_quote;

/// Finite typed runner command shared by GitHub rendering and local
/// executable orchestration tests (M002a section 7).
///
/// The GitHub renderer serializes a command to shell; tests invoke the same
/// structured command directly. The command enum remains finite to Eggpack's
/// internal CI operations; no generic arbitrary command DSL is authorized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerCommand {
    /// Verify checked-out HEAD equals the declared ReleasePlan source revision.
    VerifySource {
        /// Repository-relative ReleasePlan JSON path.
        release_plan: String,
        /// Runtime draft policy for mapped mode, omitted in legacy exact mode.
        github_policy: Option<String>,
        /// Event-selected exact source tag for mapped mode.
        expected_tag: Option<String>,
        /// Explicit checked-out repository root for mapped mode.
        source_root: Option<String>,
    },
    /// Invoke `_capture-build` for one target.
    CaptureBuild {
        /// Repository-relative contract path (informational; CLI resolves via plan).
        contract: String,
        /// Repository-relative release plan path.
        release_plan: String,
        /// Repository-relative build bindings path.
        build_bindings: String,
        /// Canonical target triple.
        target: String,
        /// Known Cargo target root on the runner.
        cargo_target_dir: String,
        /// Canonical per-target output directory to stage.
        output_dir: String,
    },
    /// Invoke `_qualify-target` for one target.
    QualifyTarget {
        /// Repository-relative contract path.
        contract: String,
        /// Repository-relative release plan path.
        release_plan: String,
        /// Repository-relative build bindings path.
        build_bindings: String,
        /// Repository-relative qualification bindings path.
        qualification_bindings: String,
        /// Canonical target triple.
        target: String,
        /// Directory holding staged candidate bytes.
        candidate_dir: String,
        /// Canonical build handoff document path.
        build_handoff: String,
        /// Canonical qualification output directory.
        output_dir: String,
        /// Optional explicit QEMU sysroot for Emulated targets.
        qemu_sysroot: Option<String>,
    },
    /// Invoke `_evaluate-gate` over downloaded qualification evidence.
    EvaluateGate {
        /// Repository-relative CI plan path.
        ci_plan: String,
        /// Canonical per-target inputs directory.
        inputs_dir: String,
        /// Gate outcome file to write.
        output: String,
    },
    /// Invoke `_aggregate` over canonical per-target directories.
    Aggregate {
        /// Repository-relative contract path.
        contract: String,
        /// Repository-relative release plan path.
        release_plan: String,
        /// Repository-relative CI plan path.
        ci_plan: String,
        /// Canonical per-target inputs directory.
        inputs_dir: String,
        /// Private finalization output root.
        output_root: String,
        /// Aggregate summary file to write.
        output: String,
    },
    /// Invoke `_prepare-stage` to materialize the staging payload.
    PrepareStage {
        /// Repository-relative contract path.
        contract: String,
        /// Internal finalized `release-manifest.json` path from aggregate.
        release_manifest: String,
        /// Internal finalized root directory from aggregate.
        finalized_root: String,
        /// Repository-relative GitHub draft policy path.
        github_policy: String,
        /// Repository-relative install policy path.
        install_policy: String,
        /// Private staging output directory.
        output_dir: String,
        /// Staging payload JSON to write.
        output_payload: String,
        /// Optional repository-relative installer presentation path (M003d).
        installer_presentation: Option<String>,
        /// Optional checked-out source root for product wrappers (M003d).
        source_root: Option<String>,
    },
    /// Invoke `_stage-github-draft` to reconcile the payload into a draft.
    StageGithubDraft {
        /// Staging payload JSON path.
        payload: String,
        /// Repository-relative GitHub draft policy path.
        github_policy: String,
        /// Private staging directory holding payload files.
        staging_dir: String,
        /// Staging receipt JSON to write.
        output_receipt: String,
    },
    /// Invoke `_validate-consumer` for one target's exact candidate.
    ValidateConsumer {
        /// Repository-relative consumer validator map path.
        consumer_validators: String,
        /// Canonical target triple.
        target: String,
        /// Directory holding staged candidate bytes.
        candidate_dir: String,
        /// Canonical build handoff document path.
        build_handoff: String,
        /// Canonical qualification evidence path (exact size/SHA source).
        evidence: String,
        /// Checked-out repository root for validator script resolution.
        source_root: String,
        /// Consumer evidence file to write.
        output: String,
    },
    /// Invoke `_resolve-release` to materialize invocation-local identity.
    ResolveRelease {
        /// Repository-relative DistributionContract TOML path.
        contract: String,
        /// Repository-relative PackConfig (TOML or JSON) path.
        pack_config: String,
        /// Repository-relative build bindings path.
        build_bindings: String,
        /// Repository-relative qualification bindings path.
        qualification_bindings: String,
        /// Optional repository-relative consumer validator map path.
        consumer_validators: Option<String>,
        /// Comma-separated contract target aliases, in order.
        selected: String,
        /// Exact existing tag (or the workflow tag expression at render).
        tag: String,
        /// Checked-out HEAD revision (or the `$head_sha` render marker).
        source_revision: String,
        /// Repository-relative GitHubDraftTemplateV1 JSON path.
        template: String,
        /// Checked-out repository root for HEAD verification.
        source_root: String,
        /// Invocation-local ReleasePlan path to write.
        output_plan: String,
        /// Invocation-local ReleaseCIPlanV1 path to write.
        output_ci_plan: String,
        /// Invocation-local GitHubDraftPolicyV1 path to write.
        output_github_policy: String,
        /// Checked-in finite identity mapping; absent preserves exact-tag bytes.
        identity_mode: Option<ReleaseIdentityMode>,
    },
}

impl RunnerCommand {
    /// Shell tokens (`eggpack`, `ci`, subcommand, flags) for YAML rendering.
    pub fn argv(&self) -> Vec<String> {
        match self {
            RunnerCommand::VerifySource {
                release_plan,
                github_policy,
                expected_tag,
                source_root,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_verify-source".into(),
                    "--release-plan".into(),
                    release_plan.clone(),
                ];
                if let (Some(policy), Some(tag)) = (github_policy, expected_tag) {
                    args.extend([
                        "--github-policy".into(),
                        policy.clone(),
                        "--expected-tag".into(),
                        tag.clone(),
                    ]);
                    if let Some(root) = source_root {
                        args.extend(["--source-root".into(), root.clone()]);
                    }
                }
                args
            }
            RunnerCommand::CaptureBuild {
                contract: _,
                release_plan,
                build_bindings,
                target,
                cargo_target_dir,
                output_dir,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_capture-build".into(),
                "--release-plan".into(),
                release_plan.clone(),
                "--build-bindings".into(),
                build_bindings.clone(),
                "--target".into(),
                target.clone(),
                "--cargo-target-dir".into(),
                cargo_target_dir.clone(),
                "--output-dir".into(),
                output_dir.clone(),
            ],
            RunnerCommand::QualifyTarget {
                contract,
                release_plan,
                build_bindings,
                qualification_bindings,
                target,
                candidate_dir,
                build_handoff,
                output_dir,
                qemu_sysroot,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_qualify-target".into(),
                    "--contract".into(),
                    contract.clone(),
                    "--release-plan".into(),
                    release_plan.clone(),
                    "--build-bindings".into(),
                    build_bindings.clone(),
                    "--qualification-bindings".into(),
                    qualification_bindings.clone(),
                    "--target".into(),
                    target.clone(),
                    "--candidate-dir".into(),
                    candidate_dir.clone(),
                    "--build-handoff".into(),
                    build_handoff.clone(),
                    "--output-dir".into(),
                    output_dir.clone(),
                ];
                if let Some(sysroot) = qemu_sysroot {
                    args.push("--qemu-sysroot".into());
                    args.push(sysroot.clone());
                }
                args
            }
            RunnerCommand::EvaluateGate {
                ci_plan,
                inputs_dir,
                output,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_evaluate-gate".into(),
                "--ci-plan".into(),
                ci_plan.clone(),
                "--inputs-dir".into(),
                inputs_dir.clone(),
                "--output".into(),
                output.clone(),
            ],
            RunnerCommand::Aggregate {
                contract,
                release_plan,
                ci_plan,
                inputs_dir,
                output_root,
                output,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_aggregate".into(),
                "--contract".into(),
                contract.clone(),
                "--release-plan".into(),
                release_plan.clone(),
                "--ci-plan".into(),
                ci_plan.clone(),
                "--inputs-dir".into(),
                inputs_dir.clone(),
                "--output-root".into(),
                output_root.clone(),
                "--output".into(),
                output.clone(),
            ],
            RunnerCommand::PrepareStage {
                contract,
                release_manifest,
                finalized_root,
                github_policy,
                install_policy,
                output_dir,
                output_payload,
                installer_presentation,
                source_root,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_prepare-stage".into(),
                    "--contract".into(),
                    contract.clone(),
                    "--release-manifest".into(),
                    release_manifest.clone(),
                    "--finalized-root".into(),
                    finalized_root.clone(),
                    "--github-policy".into(),
                    github_policy.clone(),
                    "--install-policy".into(),
                    install_policy.clone(),
                    "--output-dir".into(),
                    output_dir.clone(),
                    "--output-payload".into(),
                    output_payload.clone(),
                ];
                // M003d installer presentation is additive: absent for
                // M003c-compatible invocations, explicit otherwise.
                if let Some(presentation) = installer_presentation {
                    args.push("--installer-presentation".into());
                    args.push(presentation.clone());
                }
                if let Some(root) = source_root {
                    args.push("--source-root".into());
                    args.push(root.clone());
                }
                args
            }
            RunnerCommand::StageGithubDraft {
                payload,
                github_policy,
                staging_dir,
                output_receipt,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_stage-github-draft".into(),
                "--payload".into(),
                payload.clone(),
                "--github-policy".into(),
                github_policy.clone(),
                "--staging-dir".into(),
                staging_dir.clone(),
                "--output-receipt".into(),
                output_receipt.clone(),
            ],
            RunnerCommand::ValidateConsumer {
                consumer_validators,
                target,
                candidate_dir,
                build_handoff,
                evidence,
                source_root,
                output,
            } => vec![
                "eggpack".into(),
                "ci".into(),
                "_validate-consumer".into(),
                "--consumer-validators".into(),
                consumer_validators.clone(),
                "--target".into(),
                target.clone(),
                "--candidate-dir".into(),
                candidate_dir.clone(),
                "--build-handoff".into(),
                build_handoff.clone(),
                "--evidence".into(),
                evidence.clone(),
                "--source-root".into(),
                source_root.clone(),
                "--output".into(),
                output.clone(),
            ],
            RunnerCommand::ResolveRelease {
                contract,
                pack_config,
                build_bindings,
                qualification_bindings,
                consumer_validators,
                selected,
                tag,
                source_revision,
                template,
                source_root,
                output_plan,
                output_ci_plan,
                output_github_policy,
                identity_mode,
            } => {
                let mut args = vec![
                    "eggpack".into(),
                    "ci".into(),
                    "_resolve-release".into(),
                    "--contract".into(),
                    contract.clone(),
                    "--pack-config".into(),
                    pack_config.clone(),
                    "--build-bindings".into(),
                    build_bindings.clone(),
                    "--qualification-bindings".into(),
                    qualification_bindings.clone(),
                    "--selected".into(),
                    selected.clone(),
                    "--tag".into(),
                    tag.clone(),
                    "--source-revision".into(),
                    source_revision.clone(),
                    "--template".into(),
                    template.clone(),
                    "--source-root".into(),
                    source_root.clone(),
                    "--output-plan".into(),
                    output_plan.clone(),
                    "--output-ci-plan".into(),
                    output_ci_plan.clone(),
                    "--output-github-policy".into(),
                    output_github_policy.clone(),
                ];
                if let Some(mode) = identity_mode {
                    args.extend([
                        "--identity-mode".into(),
                        serde_json::to_value(mode)
                            .ok()
                            .and_then(|value| value.as_str().map(str::to_owned))
                            .unwrap_or_else(|| "invalid".to_owned()),
                    ]);
                }
                if let Some(map) = consumer_validators {
                    // Insert the optional map path in flag order (after
                    // qualification bindings, before selected).
                    let position = args
                        .iter()
                        .position(|arg| arg == "--selected")
                        .unwrap_or(args.len());
                    args.splice(
                        position..position,
                        ["--consumer-validators".into(), map.clone()],
                    );
                }
                args
            }
        }
    }

    /// Deterministic shell rendering with single-quote escaping.
    ///
    /// The exact token `$head_sha` is the renderer-only workflow marker for
    /// the derived HEAD revision and renders double-quoted so the runner
    /// shell expands it. It is never a valid revision, so it cannot collide
    /// with real `_resolve-release` values (which must be 40-hex).
    pub fn to_shell(&self) -> String {
        self.argv()
            .iter()
            .map(|arg| {
                if arg == "$head_sha" {
                    "\"$head_sha\"".to_owned()
                } else if arg == "$release_tag" {
                    "\"$EGGPACK_RELEASE_TAG\"".to_owned()
                } else {
                    shell_quote(arg)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
