// SPDX-License-Identifier: Apache-2.0

//! Discovery of the `.meltemi/` tree (design M2/M3).
//!
//! Walks a repository's `.meltemi/` directory into the model: constitution,
//! rumbo files, living-truth specs, and change directories (active and
//! archived). A missing `.meltemi/` yields an empty tree, not an error.

use std::path::Path;

use crate::frontmatter::parse_rumbo_file;
use crate::model::{ChangeDir, MeltemiTree, Spec};
use crate::spec_parser::parse_spec_file;

impl MeltemiTree {
    /// Discovers the `.meltemi/` tree under `repo_root`. An absent `.meltemi/`
    /// produces an empty (`exists = false`) tree.
    pub fn discover(repo_root: &Path) -> std::io::Result<Self> {
        let root = repo_root.join(".meltemi");
        if !root.is_dir() {
            return Ok(Self {
                root,
                exists: false,
                ..Self::default()
            });
        }

        let constitution = {
            let c = root.join("constitution.md");
            c.is_file().then_some(c)
        };

        let mut rumbo = Vec::new();
        let rumbo_dir = root.join("rumbo");
        if rumbo_dir.is_dir() {
            for md in markdown_files(&rumbo_dir)? {
                rumbo.push(parse_rumbo_file(&md)?);
            }
        }

        let specs = discover_specs(&root.join("specs"))?;

        let changes_dir = root.join("changes");
        let mut changes = Vec::new();
        let mut archive = Vec::new();
        if changes_dir.is_dir() {
            for dir in subdirectories(&changes_dir)? {
                let name = file_name(&dir);
                if name == "archive" {
                    for change in subdirectories(&dir)? {
                        archive.push(read_change(&change)?);
                    }
                } else {
                    changes.push(read_change(&dir)?);
                }
            }
        }

        Ok(Self {
            root,
            exists: true,
            constitution,
            rumbo,
            specs,
            changes,
            archive,
        })
    }
}

/// Parses every `<specs_dir>/<capability>/spec.md`.
fn discover_specs(specs_dir: &Path) -> std::io::Result<Vec<Spec>> {
    let mut specs = Vec::new();
    if specs_dir.is_dir() {
        for capability_dir in subdirectories(specs_dir)? {
            let spec_file = capability_dir.join("spec.md");
            if spec_file.is_file() {
                specs.push(parse_spec_file(&spec_file)?);
            }
        }
    }
    Ok(specs)
}

/// Reads one change directory, parsing its delta specs.
fn read_change(dir: &Path) -> std::io::Result<ChangeDir> {
    Ok(ChangeDir {
        name: file_name(dir),
        path: dir.to_path_buf(),
        deltas: discover_specs(&dir.join("specs"))?,
    })
}

/// Immediate subdirectories of `dir`, sorted for determinism.
fn subdirectories(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut dirs: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// `*.md` files directly in `dir`, sorted for determinism.
fn markdown_files(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    files.sort();
    Ok(files)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_meltemi_yields_empty_tree() {
        let dir = std::env::temp_dir().join(format!("meltemi-spec-empty-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tree = MeltemiTree::discover(&dir).unwrap();
        assert!(!tree.exists);
        assert!(tree.constitution.is_none());
        assert!(tree.specs.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn discovers_constitution_rumbo_and_specs() {
        let base = std::env::temp_dir().join(format!("meltemi-spec-tree-{}", std::process::id()));
        let meltemi = base.join(".meltemi");
        std::fs::create_dir_all(meltemi.join("rumbo")).unwrap();
        std::fs::create_dir_all(meltemi.join("specs").join("my-cap")).unwrap();
        std::fs::write(meltemi.join("constitution.md"), "# c\n").unwrap();
        std::fs::write(
            meltemi.join("rumbo").join("product.md"),
            "---\ninclusion: siempre\n---\nbody\n",
        )
        .unwrap();
        std::fs::write(
            meltemi.join("specs").join("my-cap").join("spec.md"),
            "### Requirement: R\n#### Scenario: S\n- **WHEN** a\n- **THEN** b\n",
        )
        .unwrap();

        let tree = MeltemiTree::discover(&base).unwrap();
        assert!(tree.exists);
        assert!(tree.constitution.is_some());
        assert_eq!(tree.rumbo.len(), 1);
        assert_eq!(tree.specs.len(), 1);
        assert_eq!(tree.specs[0].capability, "my-cap");
        assert_eq!(tree.specs[0].requirements.len(), 1);

        std::fs::remove_dir_all(&base).ok();
    }
}
