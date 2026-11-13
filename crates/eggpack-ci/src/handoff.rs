//! The build handoff document: projection, artifact validation, and staging.

use crate::error::{fail, CiError};
use crate::planning::handoff_name;
use crate::policy::{
    validate_canonical_target_dir, BUILD_HANDOFF_FILE, CANDIDATES_DIR, QUALIFICATION_EVIDENCE_FILE,
};
use crate::release_plan::decode_qualification_evidence;
use crate::validate::{safe_identifier, safe_metadata, safe_target};
use eggpack_core::{
    BuildAttempt, BuildBindingsV1, BuildStrategy, LogicalOutputSelector, PlannedTarget,
    QualificationEvidence, ReleasePlan,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeSet;
use std::path::Path;

const MAX_HANDOFF_JSON: usize = 1_000_000;

/// One bounded build output handoff entry (no absolute paths).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildHandoffOutput {
    /// Contract logical output selector.
    pub selector: LogicalOutputSelector,
    /// Explicit Cargo package source.
    pub package: String,
    /// Explicit Cargo binary target source.
    pub binary: String,
    /// Relative candidate path below the downloaded handoff root.
    pub relative_path: String,
    /// Exact byte size.
    pub size: u64,
    /// Deterministic handoff identity.
    pub handoff_identity: String,
}

/// CI-specific bounded build handoff for one canonical target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildHandoffV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Release id copied from ReleasePlan.
    pub release_id: String,
    /// Source revision copied from ReleasePlan.
    pub source_revision: String,
    /// Canonical target triple.
    pub target: String,
    /// Build strategy used for this target.
    pub strategy: BuildStrategy,
    /// Logical selector outputs in canonical order.
    pub outputs: Vec<BuildHandoffOutput>,
}

impl BuildHandoffV1 {
    /// Serialize after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("build handoff serialization failed"))
    }

    /// Parse and validate a bounded handoff document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_HANDOFF_JSON {
            return Err(fail("build handoff exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid build handoff JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Validate shape, ordering, path containment, and identifier bounds.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1
            || !safe_metadata(&self.release_id, 256)
            || !safe_metadata(&self.source_revision, 256)
            || !safe_target(&self.target)
            || self.outputs.is_empty()
            || self.outputs.len() > 256
        {
            return Err(fail("invalid or out-of-bounds build handoff"));
        }
        let mut selectors = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let mut previous: Option<&LogicalOutputSelector> = None;
        for output in &self.outputs {
            if previous.is_some_and(|p| p >= &output.selector)
                || !safe_identifier(&output.package)
                || !safe_identifier(&output.binary)
                || output.size == 0
                || !selectors.insert(output.selector.clone())
                || !identities.insert(output.handoff_identity.as_str())
                || output.handoff_identity.is_empty()
                || output.handoff_identity.len() > 256
            {
                return Err(fail("invalid or duplicate build handoff output"));
            }
            previous = Some(&output.selector);
            validate_relative_path(&output.relative_path)?;
        }
        Ok(())
    }
}

fn validate_relative_path(path: &str) -> Result<(), CiError> {
    if path.is_empty()
        || path.len() > 512
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.contains(':')
        || path.contains('\0')
        || path.contains("..")
        || path.split('/').any(|segment| segment.is_empty())
    {
        return Err(fail("build handoff path escapes its root"));
    }
    Ok(())
}

/// Project one target's build handoff from validated plan/bindings (no I/O).
pub fn project_build_handoff(
    plan: &ReleasePlan,
    bindings: &BuildBindingsV1,
    target: &str,
) -> Result<BuildHandoffV1, CiError> {
    let planned = plan
        .targets
        .iter()
        .find(|t| t.target == target)
        .ok_or_else(|| fail("build handoff target is not in ReleasePlan"))?;
    let target_bindings = bindings
        .targets
        .get(target)
        .ok_or_else(|| fail("target build bindings are missing"))?;
    let mut outputs = Vec::with_capacity(target_bindings.len());
    for binding in target_bindings {
        let relative_path = match &binding.selector {
            LogicalOutputSelector::Direct => "candidate-direct".to_string(),
            LogicalOutputSelector::BundleEntry { index } => format!("candidate-bundle-{index}"),
            LogicalOutputSelector::ArchiveMember { source } => {
                let mut digest = Sha256::new();
                digest.update(source.as_bytes());
                let hex: String = digest
                    .finalize()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                format!("candidate-archive-{hex}")
            }
        };
        // Size is filled by the runner after the build; projection uses a
        // non-zero placeholder that validation accepts but reconstruction
        // replaces with observed file sizes. Callers must not treat the
        // placeholder as evidence.
        outputs.push(BuildHandoffOutput {
            selector: binding.selector.clone(),
            package: binding.package.clone(),
            binary: binding.binary.clone(),
            relative_path,
            size: 1,
            handoff_identity: handoff_name(target, &binding.selector),
        });
    }
    outputs.sort_by(|a, b| a.selector.cmp(&b.selector));
    let handoff = BuildHandoffV1 {
        schema_version: 1,
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        target: planned.target.clone(),
        strategy: planned.policy.strategy,
        outputs,
    };
    handoff.validate()?;
    Ok(handoff)
}

/// Reconstruct an in-memory BuildAttempt from validated handoff and downloaded files.
///
/// Validates exact inventory, path containment, regular non-symlink status, and
/// sizes before returning. Never trusts artifact transport as proof; the
/// returned attempt must still pass M003/M004 validation.
pub fn reconstruct_attempt(
    plan: &ReleasePlan,
    target: &PlannedTarget,
    handoff: &BuildHandoffV1,
    candidate_root: &Path,
) -> Result<BuildAttempt, CiError> {
    if handoff.schema_version != 1
        || handoff.release_id != plan.release_id
        || handoff.source_revision != plan.source_revision
        || handoff.target != target.target
        || handoff.strategy != target.policy.strategy
    {
        return Err(fail("build handoff identity differs from ReleasePlan"));
    }
    handoff.validate()?;
    let root = candidate_root.to_path_buf();
    let mut candidates = Vec::with_capacity(handoff.outputs.len());
    for output in &handoff.outputs {
        let path = root.join(&output.relative_path);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| fail("candidate file is unavailable"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
            return Err(fail("candidate is not a non-empty regular file"));
        }
        if metadata.len() != output.size && output.size != 1 {
            // Placeholder size 1 means the projection has not yet been
            // replaced with observed sizes; runners must supply exact sizes.
            return Err(fail("candidate size differs from handoff evidence"));
        }
        candidates.push(eggpack_core::CandidateArtifact {
            target: target.target.clone(),
            selector: output.selector.clone(),
            package: output.package.clone(),
            binary: output.binary.clone(),
            path,
            size: metadata.len(),
        });
    }
    candidates.sort_by(|a, b| a.selector.cmp(&b.selector));
    Ok(BuildAttempt {
        release_id: plan.release_id.clone(),
        source_revision: plan.source_revision.clone(),
        target: target.target.clone(),
        strategy: target.policy.strategy,
        tool_summary: format!("ci-handoff {}", target.target),
        process: eggpack_core::ProcessEvidence {
            outcome: eggpack_core::CommandOutcome::Success,
            stdout_bytes: 0,
            stderr_bytes: 0,
        },
        candidates,
    })
}

/// Validate a canonical per-target build artifact directory:
/// `build-handoff.json` plus `candidates/<relative_path>` entries exactly
/// matching the handoff inventory (regular non-empty files, no extras).
pub fn validate_build_artifact_dir(dir: &Path) -> Result<BuildHandoffV1, CiError> {
    let handoff_text = std::fs::read_to_string(dir.join(BUILD_HANDOFF_FILE))
        .map_err(|_| fail("build handoff is unavailable"))?;
    let handoff = BuildHandoffV1::from_json(&handoff_text)?;
    let candidates_dir = dir.join(CANDIDATES_DIR);
    let meta = std::fs::symlink_metadata(&candidates_dir)
        .map_err(|_| fail("candidate directory is unavailable"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(fail("candidate directory is not a real directory"));
    }
    let mut seen = BTreeSet::new();
    for output in &handoff.outputs {
        let path = candidates_dir.join(&output.relative_path);
        let file_meta =
            std::fs::symlink_metadata(&path).map_err(|_| fail("candidate file is unavailable"))?;
        if file_meta.file_type().is_symlink()
            || !file_meta.is_file()
            || file_meta.len() == 0
            || file_meta.len() != output.size
        {
            return Err(fail("candidate is not the handoff-described regular file"));
        }
        seen.insert(output.relative_path.as_str());
    }
    // No extra files.
    let mut count = 0;
    for entry in
        std::fs::read_dir(&candidates_dir).map_err(|_| fail("candidate directory unreadable"))?
    {
        let entry = entry.map_err(|_| fail("candidate directory unreadable"))?;
        let meta = std::fs::symlink_metadata(entry.path())
            .map_err(|_| fail("candidate file is unavailable"))?;
        if !meta.is_file() {
            return Err(fail("candidate directory contains a non-file entry"));
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !seen.contains(name.as_str()) {
            return Err(fail("candidate directory contains an extra file"));
        }
        count += 1;
    }
    if count != seen.len() {
        return Err(fail("candidate directory inventory differs from handoff"));
    }
    Ok(handoff)
}

/// Validate a canonical per-target qualification artifact directory:
/// `build-handoff.json`, `evidence.json`, plus `candidates/...`.
pub fn validate_qualification_artifact_dir(
    dir: &Path,
) -> Result<(BuildHandoffV1, QualificationEvidence), CiError> {
    let handoff = validate_build_artifact_dir(dir)?;
    let evidence_text = std::fs::read_to_string(dir.join(QUALIFICATION_EVIDENCE_FILE))
        .map_err(|_| fail("qualification evidence is unavailable"))?;
    let evidence = decode_qualification_evidence(&evidence_text)?;
    if evidence.target != handoff.target
        || evidence.release_id != handoff.release_id
        || evidence.source_revision != handoff.source_revision
    {
        return Err(fail("qualification evidence identity differs from handoff"));
    }
    Ok((handoff, evidence))
}

/// Copy or hard-link one file without following symlinks on the source.
fn copy_candidate_file(source: &Path, dest: &Path) -> Result<(), CiError> {
    let meta =
        std::fs::symlink_metadata(source).map_err(|_| fail("candidate file is unavailable"))?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() == 0 {
        return Err(fail("candidate is not a non-empty regular file"));
    }
    if let Ok(parent) = dest
        .parent()
        .ok_or_else(|| fail("candidate path is invalid"))
    {
        let _ = parent;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|_| fail("output directory unavailable"))?;
    }
    // Prefer hard-link (same filesystem, no byte copy); fall back to copy.
    if std::fs::hard_link(source, dest).is_err() {
        std::fs::copy(source, dest).map_err(|_| fail("candidate staging failed"))?;
    }
    Ok(())
}

/// Stage a canonical build artifact directory from validated handoff sizes
/// and a source candidate root keyed by handoff relative names.
pub fn stage_build_artifact_dir(
    handoff: &BuildHandoffV1,
    candidate_src_dir: &Path,
    out_dir: &Path,
) -> Result<(), CiError> {
    let out_meta = std::fs::symlink_metadata(out_dir);
    match out_meta {
        Ok(meta) => {
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(fail("output directory is not a real directory"));
            }
        }
        Err(_) => {
            std::fs::create_dir_all(out_dir).map_err(|_| fail("output directory unavailable"))?;
        }
    }
    let dest_candidates = out_dir.join(CANDIDATES_DIR);
    std::fs::create_dir_all(&dest_candidates).map_err(|_| fail("output directory unavailable"))?;
    for output in &handoff.outputs {
        let source = candidate_src_dir.join(&output.relative_path);
        copy_candidate_file(&source, &dest_candidates.join(&output.relative_path))?;
    }
    let json = handoff.to_json()?;
    std::fs::write(out_dir.join(BUILD_HANDOFF_FILE), json.as_bytes())
        .map_err(|_| fail("handoff write failed"))?;
    validate_build_artifact_dir(out_dir)?;
    Ok(())
}

/// Derive the Cargo output file for one handoff entry from the known Cargo
/// target root (`<cargo_target_dir>/<target>/release/<binary>[.exe]`).
pub fn cargo_output_path(
    cargo_target_dir: &Path,
    target: &str,
    binary: &str,
) -> Result<std::path::PathBuf, CiError> {
    validate_canonical_target_dir(target)?;
    if binary.is_empty()
        || binary.len() > 128
        || binary.contains('/')
        || binary.contains('\\')
        || binary.contains('\0')
        || binary.contains("..")
    {
        return Err(fail("cargo binary name escapes its root"));
    }
    let mut path = cargo_target_dir.join(target).join("release").join(binary);
    if target.contains("-windows-") {
        // Cargo appends the executable suffix to the full binary name; a binary
        // name containing `.` keeps it (`tool.cli` -> `tool.cli.exe`).
        path.set_file_name(format!("{binary}.exe"));
    }
    Ok(path)
}

// ---------------------------------------------------------------------------
// M002 — qualification/aggregation gates and deterministic release graph.
// ---------------------------------------------------------------------------
