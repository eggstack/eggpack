//! The one workflow drift comparison and the three public check functions that use it.

use crate::error::{fail, CiError};
use crate::identity::ReleaseWorkflowShapeV1;
use crate::planning::CIPlan;
use crate::policy::GitHubPolicy;
use crate::release_plan::ReleaseCIPlanV1;
use crate::render_github::render_github;
use crate::render_release::{render_release_github, render_reusable_release_github};
use eggpack_contract::DistributionContract;

pub(crate) const MAX_WORKFLOW_BYTES: usize = 8_000_000;

/// Deterministic check result; original workflow bytes are never returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    /// Whether normalized existing bytes match rendered output.
    pub matches: bool,
    /// Expected byte count after newline normalization.
    pub expected_bytes: usize,
    /// Existing byte count after newline normalization.
    pub actual_bytes: usize,
    /// First differing byte offset, if any.
    pub first_difference: Option<usize>,
}

/// Compare already-rendered expected bytes with existing workflow bytes.
///
/// The size bound, the CRLF-to-LF-only normalization, and the
/// first-difference computation are defined once here, so every check
/// function behaves identically on identical inputs. The expected bytes are
/// already rendered by the caller: this function never renders.
pub(crate) fn compare_workflow_bytes(
    expected: &[u8],
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    if existing.len() > MAX_WORKFLOW_BYTES {
        return Err(fail("existing workflow exceeds size bound"));
    }
    let expected = normalize_newlines(expected);
    let existing = normalize_newlines(existing);
    let matches = expected == existing;
    let first_difference = if matches {
        None
    } else {
        Some(
            expected
                .iter()
                .zip(&existing)
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| expected.len().min(existing.len())),
        )
    };
    Ok(DriftReport {
        matches,
        expected_bytes: expected.len(),
        actual_bytes: existing.len(),
        first_difference,
    })
}

/// Compare existing workflow bytes with deterministic output without writing files.
pub fn check_github(
    plan: &CIPlan,
    policy: &GitHubPolicy,
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    let expected = render_github(plan, policy)?;
    compare_workflow_bytes(expected.as_bytes(), existing)
}

pub(crate) fn normalize_newlines(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            out.push(b'\n');
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// Compare existing reusable workflow bytes without writing files.
pub fn check_reusable_release_github(
    contract: &DistributionContract,
    shape: &ReleaseWorkflowShapeV1,
    policy: &GitHubPolicy,
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    let expected = render_reusable_release_github(contract, shape, policy)?;
    compare_workflow_bytes(expected.as_bytes(), existing)
}

/// Compare existing release workflow bytes without writing files.
pub fn check_release_github(
    graph: &ReleaseCIPlanV1,
    policy: &GitHubPolicy,
    existing: &[u8],
) -> Result<DriftReport, CiError> {
    let expected = render_release_github(graph, policy)?;
    compare_workflow_bytes(expected.as_bytes(), existing)
}
