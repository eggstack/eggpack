//! Bounded identifier, metadata, and release-identity string validators.

use crate::error::{fail, CiError};

pub(crate) fn safe_target(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub(crate) fn safe_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub(crate) fn safe_metadata(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

pub(crate) fn safe_tool_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".+-_".contains(&b))
}

pub(crate) fn validate_exact_tag(tag: &str) -> Result<(), CiError> {
    // The canonical exact-tag safety rule is owned by `eggpack-core`; this
    // crate keeps its own diagnostic wording for each refusal class.
    eggpack_core::validate_exact_tag_safety(tag).map_err(|error| {
        fail(match error {
            eggpack_core::ExactTagSafetyError::OutOfBounds => "exact tag out of bounds",
            eggpack_core::ExactTagSafetyError::ControlCharacters => {
                "exact tag contains control characters"
            }
            eggpack_core::ExactTagSafetyError::QueryOrInjectionCharacters => {
                "exact tag contains query/injection characters"
            }
            eggpack_core::ExactTagSafetyError::UnsafePathShape => "exact tag has unsafe path shape",
            eggpack_core::ExactTagSafetyError::UnsafeGitRefPath => {
                "exact tag has unsafe git ref path shape"
            }
        })
    })
}

pub(crate) fn validate_source_revision(revision: &str) -> Result<(), CiError> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fail("source revision must be 40 lowercase hex characters"));
    }
    Ok(())
}
