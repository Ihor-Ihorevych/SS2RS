//! Keeps the four lists of fuzz targets in step (T12-007): the files in `fuzz/fuzz_targets/`, the
//! `[[bin]]` entries of `fuzz/Cargo.toml`, the matrix of `.github/workflows/nightly.yml` and the
//! table of `docs/FUZZING.md`. A target that is missing from the workflow would never run at
//! night; one missing from the manifest would not build.

/// The names of the `[[bin]]` entries of a manifest, in order.
pub fn manifest_bins(manifest: &str) -> Vec<String> {
    let mut bins = Vec::new();
    let mut in_bin = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
        } else if in_bin
            && let Some(rest) = line.strip_prefix("name")
            && let Some(name) = rest.trim_start().strip_prefix('=')
        {
            bins.push(name.trim().trim_matches('"').to_owned());
            in_bin = false;
        }
    }
    bins
}

/// The entries of the `target:` list of a workflow's strategy matrix.
pub fn workflow_targets(workflow: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut listing = false;
    for line in workflow.lines() {
        let trimmed = line.trim();
        if trimmed == "target:" {
            listing = true;
        } else if listing {
            match trimmed.strip_prefix("- ") {
                Some(name) => targets.push(name.trim().to_owned()),
                None => listing = false,
            }
        }
    }
    targets
}

/// The names in the first column of a markdown table row written as `` | `name` | ``.
pub fn doc_targets(doc: &str) -> Vec<String> {
    doc.lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|rest| rest.split_once('`').map(|(name, _)| name.to_owned()))
        .filter(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn manifest_bins_are_read_from_bin_tables_only() {
        let manifest = "[package]\nname = \"ss2-fuzz\"\n\n[[bin]]\nname = \"a_one\"\npath = \"x.rs\"\n\n[dependencies]\nname = \"no\"\n\n[[bin]]\npath = \"y.rs\"\nname = \"b_two\"\n";
        assert_eq!(manifest_bins(manifest), ["a_one", "b_two"]);
    }

    #[test]
    fn workflow_targets_stop_at_the_end_of_the_list() {
        let yaml = "      matrix:\n        target:\n          - one\n          - two_x\n        other:\n          - not_this\n";
        assert_eq!(workflow_targets(yaml), ["one", "two_x"]);
    }

    #[test]
    fn doc_targets_take_the_first_column_of_code_names() {
        let doc = "| Target | Entry |\n|---|---|\n| `vfs_path` | `VfsPath::new` |\n| `gro_archive` | x |\nprose `not_a_row`\n| `Mixed_Case` | skipped |\n";
        assert_eq!(doc_targets(doc), ["vfs_path", "gro_archive"]);
    }

    fn read(relative: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(relative);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    #[test]
    fn the_four_lists_of_fuzz_targets_agree() {
        let sorted = |mut v: Vec<String>| {
            v.sort();
            v
        };
        let mut files: Vec<String> =
            std::fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../fuzz/fuzz_targets"))
                .expect("fuzz/fuzz_targets exists")
                .filter_map(|e| {
                    e.ok()?
                        .file_name()
                        .to_str()?
                        .strip_suffix(".rs")
                        .map(str::to_owned)
                })
                .collect();
        files.sort();
        assert!(!files.is_empty(), "no fuzz targets found");
        let bins = sorted(manifest_bins(&read("fuzz/Cargo.toml")));
        let matrix = sorted(workflow_targets(&read(".github/workflows/nightly.yml")));
        let docs = sorted(doc_targets(&read("docs/FUZZING.md")));
        assert_eq!(
            bins, files,
            "fuzz/Cargo.toml [[bin]] entries and fuzz/fuzz_targets/*.rs differ"
        );
        assert_eq!(
            matrix, files,
            "the matrix of .github/workflows/nightly.yml and fuzz/fuzz_targets/*.rs differ"
        );
        assert_eq!(
            docs, files,
            "the table of docs/FUZZING.md and fuzz/fuzz_targets/*.rs differ"
        );
    }
}
