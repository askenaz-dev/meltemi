// SPDX-License-Identifier: Apache-2.0

//! Structured validation diagnostics (design M6).
//!
//! A diagnostic points at the artifact, line and rule that was violated. The
//! `Rule` enum is stable and machine-readable; messages are in English (the
//! engine's strings, per constitution §11).

use std::path::PathBuf;

/// A single structural-conformance problem found by the validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The artifact the problem is in.
    pub file: PathBuf,
    /// 1-based line the problem anchors to (0 when it is file-level).
    pub line: usize,
    /// The stable rule that was violated.
    pub rule: Rule,
    /// Human-readable English detail.
    pub message: String,
}

impl Diagnostic {
    pub(crate) fn new(
        file: impl Into<PathBuf>,
        line: usize,
        rule: Rule,
        message: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            rule,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{} [{}] {}",
            self.file.display(),
            self.line,
            self.rule.as_str(),
            self.message
        )
    }
}

/// The structural rules the engine checks. Machine-readable and stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// A `### Requirement:` has no recognized `#### Scenario:`.
    RequirementWithoutScenario,
    /// A `## … Requirements` delta header is not in the canon.
    UnknownDeltaHeader,
    /// A capability directory name is not kebab-case.
    NonKebabCapability,
    /// A rumbo file has no front-matter or no `inclusion` key.
    MissingFrontMatter,
    /// `inclusion: por-patrón` without a non-empty `fileMatch`.
    OnMatchWithoutFileMatch,
}

impl Rule {
    /// The stable machine-readable slug for this rule.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequirementWithoutScenario => "requirement-without-scenario",
            Self::UnknownDeltaHeader => "unknown-delta-header",
            Self::NonKebabCapability => "non-kebab-capability",
            Self::MissingFrontMatter => "missing-front-matter",
            Self::OnMatchWithoutFileMatch => "on-match-without-file-match",
        }
    }
}
