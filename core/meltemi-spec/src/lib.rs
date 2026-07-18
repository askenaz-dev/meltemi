// SPDX-License-Identifier: Apache-2.0

//! Parser and structural validator for Meltemi's `.meltemi/` artifacts.
//!
//! This crate is the first implementation of the `artifact-format` contract:
//! it discovers the `.meltemi/` tree, parses specs and rumbo front-matter into
//! an in-memory [`model`], and validates the *structural* subset of the format
//! ([`validate`]), reporting located [`Diagnostic`]s. EARS-keyword validation
//! and delta application are built on top of this in later changes.
//!
//! ```no_run
//! use meltemi_spec::{MeltemiTree, validate_tree};
//! let tree = MeltemiTree::discover(std::path::Path::new(".")).unwrap();
//! let diagnostics = validate_tree(&tree);
//! assert!(diagnostics.is_empty(), "artifacts are non-conformant: {diagnostics:?}");
//! ```

pub mod delta;
pub mod diagnostic;
pub mod ears;
pub mod frontmatter;
pub mod model;
pub mod project;
pub mod spec_parser;
pub mod tree;
pub mod validate;

pub use delta::{DeltaOp, DeltaSpec, apply_delta, parse_delta};
pub use diagnostic::{Diagnostic, Rule};
pub use model::{
    ChangeDir, DeltaOperation, DeltaSection, Inclusion, MeltemiTree, Ratification, Requirement,
    RumboFile, Scenario, Spec, Step, StepMarker,
};
pub use project::{ActiveChange, ProjectionSources, project};
pub use spec_parser::{parse_spec, parse_spec_file};
pub use validate::{validate_rumbo, validate_spec, validate_tree};
