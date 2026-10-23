//! The static workflow shape and the runtime release identity it resolves to.

use crate::consumer_validator::ConsumerValidatorV1;
use crate::error::{fail, CiError};
use crate::policy::{ReleaseIdentityMode, StagingTagSource};
use crate::release_plan::StagingProvider;
use crate::validate::{safe_metadata, safe_target, validate_exact_tag, validate_source_revision};
use eggpack_contract::DistributionContract;
use eggpack_core::{
    BuildBindingsV1, PackConfig, QualificationBindingsV1, ReleasePlan, TargetPolicy,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Workflow-private runtime identity directory (invocation-local storage).
pub const RUNTIME_IDENTITY_DIR: &str = "eggpack-runtime";

/// Workflow-private runtime ReleasePlan path.
pub const RUNTIME_RELEASE_PLAN: &str = "eggpack-runtime/release-plan.json";

/// Workflow-private runtime ReleaseCIPlanV1 path.
pub const RUNTIME_CI_PLAN: &str = "eggpack-runtime/release-ci-plan.json";

/// Workflow-private runtime GitHubDraftPolicyV1 path.
pub const RUNTIME_GITHUB_POLICY: &str = "eggpack-runtime/github-draft.json";

/// Internal preflight artifact carrying the runtime identity documents.
pub const RUNTIME_IDENTITY_ARTIFACT: &str = "eggpack-runtime-identity";

const MAX_SHAPE_JSON: usize = 1_000_000;

/// Placeholder release id used only to project the static graph structure.
///
/// It must never appear in rendered workflow bytes; reusable rendering
/// asserts its absence along with the placeholder revision.
pub(crate) const UNRESOLVED_RELEASE_ID: &str = "0.0.0-m003d-unresolved";

/// Placeholder source revision used only to project static graph structure.
pub(crate) const UNRESOLVED_SOURCE_REVISION: &str = "0000000000000000000000000000000000000000";

/// Bounded staging intent carried by the static workflow shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeStagingIntentV1 {
    /// Staging provider (exactly GitHub draft).
    pub provider: StagingProvider,
    /// Explicit tag source mapping.
    pub tag_source: StagingTagSource,
    /// Whether staging is required.
    pub required: bool,
}

/// Static checked-in release workflow shape (M003d section 4A).
///
/// Contains only identity-independent information needed to render the
/// checked-in workflow: canonical target set, target policy, build and
/// core qualification bindings, consumer validator configuration, and
/// staging intent. It MUST NOT contain release_id, source_revision, an
/// exact future tag, a GitHub release id, or artifact digests/sizes.
/// Historical `CIPlan` v1 / `ReleaseCIPlanV1` evidence is preserved rather
/// than silently reinterpreted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseWorkflowShapeV1 {
    /// Schema version, exactly 1.
    pub schema_version: u32,
    /// Canonical ordered PackConfig targets (identity-independent policy).
    pub targets: Vec<TargetPolicy>,
    /// Contract target aliases selected for resolution, in order.
    pub selected_aliases: Vec<String>,
    /// Explicit build bindings.
    pub build_bindings: BuildBindingsV1,
    /// Explicit core qualification bindings.
    pub qualification_bindings: QualificationBindingsV1,
    /// Per-target consumer validators (empty when disabled).
    #[serde(default)]
    pub consumer_validators: BTreeMap<String, ConsumerValidatorV1>,
    /// Staging intent. Required for reusable release rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staging: Option<ShapeStagingIntentV1>,
}

impl ReleaseWorkflowShapeV1 {
    /// Parse a strict bounded JSON shape document.
    pub fn from_json(text: &str) -> Result<Self, CiError> {
        if text.len() > MAX_SHAPE_JSON {
            return Err(fail("workflow shape exceeds size bound"));
        }
        let value: Self =
            serde_json::from_str(text).map_err(|_| fail("invalid workflow shape JSON"))?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize deterministically after validation.
    pub fn to_json(&self) -> Result<String, CiError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| fail("workflow shape encode failed"))
    }

    /// Validate schema version, canonical target ordering, alias bounds,
    /// and embedded binding/validator documents.
    pub fn validate(&self) -> Result<(), CiError> {
        if self.schema_version != 1 {
            return Err(fail("unsupported workflow shape version"));
        }
        if self.targets.is_empty() || self.targets.len() > 256 {
            return Err(fail("workflow shape target count out of bounds"));
        }
        let mut previous: Option<&str> = None;
        let mut seen = BTreeSet::new();
        for policy in &self.targets {
            if !safe_target(&policy.target)
                || !seen.insert(policy.target.as_str())
                || previous.is_some_and(|p| p >= policy.target.as_str())
            {
                return Err(fail(
                    "workflow shape targets must be unique, canonical, and ordered",
                ));
            }
            previous = Some(&policy.target);
        }
        if self.selected_aliases.is_empty() || self.selected_aliases.len() > 256 {
            return Err(fail("workflow shape alias count out of bounds"));
        }
        for alias in &self.selected_aliases {
            if !safe_metadata(alias, 128) {
                return Err(fail("workflow shape alias out of bounds"));
            }
        }
        for validator in self.consumer_validators.values() {
            validator.validate()?;
        }
        if let Some(staging) = &self.staging {
            if staging.provider != StagingProvider::GitHubDraft {
                return Err(fail("unsupported shape staging provider"));
            }
        }
        Ok(())
    }

    /// View the shape targets as a PackConfig for canonical resolution.
    pub fn pack_config(&self) -> PackConfig {
        PackConfig {
            schema_version: 1,
            targets: self.targets.clone(),
        }
    }
}

/// Resolve one invocation's exact release identity from static configuration.
///
/// Validates the exact existing tag selected by the workflow event/input,
/// consumes the checked-out HEAD revision, resolves PackConfig against
/// DistributionContract through `PackConfig::resolve`, and uses the exact
/// tag as the opaque `release_id` (no product-specific transformation).
///
/// Resolution is pure: no clock, no environment reads, and no GitHub or
/// network access. GitHub appears in this function only through the caller's
/// separate draft-template resolution for the same tag.
pub fn resolve_runtime_release_plan(
    contract: &DistributionContract,
    pack_config: &PackConfig,
    selected: &[String],
    tag: &str,
    source_revision: &str,
) -> Result<ReleasePlan, CiError> {
    resolve_runtime_release_plan_with_mode(
        contract,
        pack_config,
        selected,
        tag,
        source_revision,
        ReleaseIdentityMode::ExactTag,
    )
}

/// Resolve a release plan using an explicit checked-in source-tag mapping.
pub fn resolve_runtime_release_plan_with_mode(
    contract: &DistributionContract,
    pack_config: &PackConfig,
    selected: &[String],
    tag: &str,
    source_revision: &str,
    mode: ReleaseIdentityMode,
) -> Result<ReleasePlan, CiError> {
    validate_exact_tag(tag)?;
    validate_source_revision(source_revision)?;
    let release_id = mode.release_id(tag)?;
    if selected.is_empty() || selected.len() > 256 {
        return Err(fail("selected target count out of bounds"));
    }
    for alias in selected {
        if !safe_metadata(alias, 128) {
            return Err(fail("selected target alias out of bounds"));
        }
    }
    if pack_config.schema_version != 1 {
        return Err(fail("unsupported PackConfig version"));
    }
    let plan = pack_config
        .resolve(contract, &release_id, source_revision, selected)
        .map_err(|_| fail("PackConfig does not resolve for the selected identity and source"))?;
    if plan.release_id != release_id || plan.source_revision != source_revision {
        return Err(fail(
            "resolved ReleasePlan differs from the selected identity and source",
        ));
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------
// M003d — static workflow shape vs runtime release identity.
// ---------------------------------------------------------------------------
