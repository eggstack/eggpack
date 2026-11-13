//! The crate's bounded failure type and its single constructor.

use std::fmt;

/// Bounded failure from CI planning, policy validation, rendering, or drift checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiError(pub(crate) String);

impl fmt::Display for CiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CiError {}

pub(crate) fn fail(message: &str) -> CiError {
    CiError(message.to_owned())
}
