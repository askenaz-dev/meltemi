// SPDX-License-Identifier: Apache-2.0

//! In-memory model of Meltemi's `.meltemi/` artifacts (design M2).
//!
//! The model is what the parser produces and the validator consumes. It is a
//! faithful, position-preserving representation of the canonical format
//! (`artifact-format`): every element keeps the 1-based line where it was
//! found, so diagnostics can point at it.

use std::path::PathBuf;

/// A parsed spec file: a living-truth spec (`specs/<capability>/spec.md`) or a
/// change's delta spec (`changes/<name>/specs/<capability>/spec.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// Capability name, taken from the containing directory (kebab-case).
    pub capability: String,
    /// Requirements declared in the file, in order.
    pub requirements: Vec<Requirement>,
    /// Delta operation section headers found (empty in a living-truth spec).
    pub deltas: Vec<DeltaSection>,
    /// The file the spec was parsed from.
    pub source: PathBuf,
}

/// A `## <OP> Requirements` section header inside a delta spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeltaSection {
    /// The recognized operation, or `None` when the header is not canonical.
    pub operation: Option<DeltaOperation>,
    /// The raw operation word as written (e.g. `ADDED`, or a non-canonical one).
    pub raw: String,
    /// 1-based line of the header.
    pub line: usize,
}

/// The canonical delta operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaOperation {
    Added,
    Modified,
    Removed,
    Renamed,
}

impl DeltaOperation {
    /// Classifies a delta operation word; `None` if outside the canon.
    #[must_use]
    pub fn classify(word: &str) -> Option<Self> {
        match word {
            "ADDED" => Some(Self::Added),
            "MODIFIED" => Some(Self::Modified),
            "REMOVED" => Some(Self::Removed),
            "RENAMED" => Some(Self::Renamed),
            _ => None,
        }
    }
}

/// A single requirement and its scenarios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// Name after `### Requirement:`.
    pub name: String,
    /// Prose between the requirement header and its first scenario.
    pub description: String,
    /// Scenarios, in order. A conformant requirement has at least one.
    pub scenarios: Vec<Scenario>,
    /// 1-based line of the `### Requirement:` header.
    pub line: usize,
}

/// A single scenario and its steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    /// Name after `#### Scenario:`.
    pub name: String,
    /// Steps, in order.
    pub steps: Vec<Step>,
    /// 1-based line of the `#### Scenario:` header.
    pub line: usize,
}

/// One `- **MARKER** text` step of a scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The classified step marker.
    pub marker: StepMarker,
    /// The step text after the marker.
    pub text: String,
}

/// EARS step markers recognized in scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMarker {
    When,
    While,
    If,
    Then,
    Where,
    And,
    /// A bolded marker outside the canon.
    Other,
}

impl StepMarker {
    /// Classifies a step marker word.
    #[must_use]
    pub fn classify(word: &str) -> Self {
        match word {
            "WHEN" => Self::When,
            "WHILE" => Self::While,
            "IF" => Self::If,
            "THEN" => Self::Then,
            "WHERE" => Self::Where,
            "AND" => Self::And,
            _ => Self::Other,
        }
    }
}

/// A `rumbo/` file with its parsed front-matter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RumboFile {
    /// The file on disk.
    pub path: PathBuf,
    /// Inclusion mode, or `None` when front-matter is missing or malformed.
    pub inclusion: Option<Inclusion>,
    /// Optional ratification metadata.
    pub ratification: Option<Ratification>,
    /// The document body (everything after the front-matter block).
    pub body: String,
}

/// When a rumbo file is injected as context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inclusion {
    /// `inclusion: siempre`.
    Always,
    /// `inclusion: por-patrón` with its `fileMatch` globs.
    OnMatch(Vec<String>),
    /// `inclusion: manual`.
    Manual,
}

/// Optional ratification stamp in a document's front-matter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ratification {
    pub date: String,
    pub ratifier: String,
}

/// A change directory under `changes/` or `changes/archive/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeDir {
    /// The change name (directory name).
    pub name: String,
    /// The directory on disk.
    pub path: PathBuf,
    /// Parsed delta specs under `specs/`.
    pub deltas: Vec<Spec>,
}

/// The discovered `.meltemi/` tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MeltemiTree {
    /// The `.meltemi/` directory (may not exist).
    pub root: PathBuf,
    /// Whether `.meltemi/` exists.
    pub exists: bool,
    /// Path to `constitution.md`, if present.
    pub constitution: Option<PathBuf>,
    /// Parsed `rumbo/` files.
    pub rumbo: Vec<RumboFile>,
    /// Living-truth specs (`specs/<capability>/spec.md`).
    pub specs: Vec<Spec>,
    /// Active changes (`changes/<name>/`).
    pub changes: Vec<ChangeDir>,
    /// Archived changes (`changes/archive/<name>/`).
    pub archive: Vec<ChangeDir>,
}
