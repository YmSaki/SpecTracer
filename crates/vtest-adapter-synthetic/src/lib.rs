//! `synthetic` — a minimal `SourceDiscoveryAdapter`-only adapter whose sole
//! purpose is to prove DS-1584「synthetic adapterは.rs以外のsource、関数
//! ではないTest construct、doc commentではないmetadata宣言、Rust item path
//! ではないopaque locatorを、vtest-model、vtest-scan、vtest-verifyの変更
//! なしで登録・scan・verifyできる」without touching core.
//!
//! This crate depends on `vtest-adapter-api` and `vtest-model` only — it
//! does not depend on `vtest-scan`, `vtest-verify`, `vtest-exec`, or
//! `vtest-adapter-rust` (完了条件: このcrateのCargo.tomlの依存はこの2つと
//! serde等の外部crateだけ)。
//!
//! # File format
//!
//! This adapter discovers `*.synthetic` files (deliberately not `.rs`).
//! Each file is a sequence of blocks separated by a `---` line. A block
//! whose first line starts with `@synthetic.test` declares a Test
//! construct; its remaining `@synthetic.<key> <value>` lines are metadata
//! (plain lines in the file body, not a doc comment — DS-1584's "doc
//! commentではないmetadata宣言"), and the block's own text (including the
//! `@synthetic.test` line itself) is the Test construct bytes. Recognized
//! keys: `id`, `covers` (comma-separated), `target` (repeatable, opaque —
//! no `::` syntax, so it is never mistaken for `rust-cargo`'s `<path>::
//! <item>` locator shape — DS-1584's "Rust item pathではないopaque
//! locator"), `intent`.
//!
//! A block whose first line is `@synthetic.source <locator>` instead
//! declares a Source Target: a construct that is not a function (DS-1584's
//! "関数ではないTest construct" — here applied symmetrically to the Source
//! Target side, since this adapter has no function concept at all) whose
//! `locator` is the block's own opaque, colon-free id.

use std::{
    fs,
    path::{Path, PathBuf},
};

use vtest_adapter_api::{
    Adapter, AdapterDescriptor, AdapterScanConfig, Capability, DiscoveryError, DiscoveryOutcome,
    SourceDiscoveryAdapter, SourceDraft, TestDraft,
};
use vtest_model::{
    AdapterId, ExecutionDescriptor, Locator, SourceLocation, TargetRef, TestId, VoId,
};

/// The id this adapter registers under, and the `TargetRef::Locator.adapter`
/// / `SourceLocation.adapter` value it stamps.
pub const ADAPTER_ID: &str = "synthetic";

/// The file extension this adapter discovers (deliberately not `.rs`).
pub const FILE_EXTENSION: &str = "synthetic";

#[derive(Default)]
pub struct SyntheticAdapter;

impl SyntheticAdapter {
    pub fn new() -> Self {
        Self
    }
}

/// This adapter declares only the discovery capability (DS-1584 scopes the
/// claim to discovery/scan/verify passing through unmodified; static
/// analysis, coverage, and running have no meaning for an adapter with no
/// real toolchain behind it). Declaring nothing else means
/// `AdapterRegistry::register` requires `as_static_analysis`/
/// `as_test_runner`/`as_coverage` to stay `None` (the trait's defaults) —
/// which they do.
impl Adapter for SyntheticAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            id: ADAPTER_ID.to_owned(),
            languages: vec!["synthetic".to_owned()],
            capabilities: vec![Capability::SourceDiscovery],
            config_namespace: ADAPTER_ID.to_owned(),
        }
    }

    fn as_source_discovery(&self) -> Option<&dyn SourceDiscoveryAdapter> {
        Some(self)
    }
}

impl SourceDiscoveryAdapter for SyntheticAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn discover(
        &self,
        root: &Path,
        fallback_package: &str,
        config: &AdapterScanConfig,
    ) -> Result<DiscoveryOutcome, DiscoveryError> {
        let mut paths = Vec::new();
        for include_path in &config.include_paths {
            let full = root.join(include_path);
            collect_synthetic_files(&full, &mut paths).map_err(|error| DiscoveryError {
                path: full.clone(),
                message: error.to_string(),
            })?;
        }
        paths.sort();
        paths.dedup();

        let mut tests = Vec::new();
        let mut sources = Vec::new();
        let mut files_scanned = 0usize;
        for path in &paths {
            files_scanned += 1;
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(path).map_err(|error| DiscoveryError {
                path: path.clone(),
                message: error.to_string(),
            })?;
            parse_file(&relative, &text, fallback_package, &mut tests, &mut sources);
        }

        Ok(DiscoveryOutcome {
            files_scanned,
            tests,
            missing_tests: Vec::new(),
            sources,
            diagnostics: Vec::new(),
        })
    }
}

fn collect_synthetic_files(path: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if path.extension().and_then(|value| value.to_str()) == Some(FILE_EXTENSION) {
            output.push(path.to_owned());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_synthetic_files(&entry_path, output)?;
        } else if entry_path.extension().and_then(|value| value.to_str()) == Some(FILE_EXTENSION) {
            output.push(entry_path);
        }
    }
    Ok(())
}

/// One `@synthetic.<key> <value>` metadata line.
fn parse_annotation(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim().strip_prefix("@synthetic.")?;
    match rest.find(char::is_whitespace) {
        Some(index) => Some((&rest[..index], rest[index..].trim())),
        None => Some((rest, "")),
    }
}

fn parse_file(
    relative: &str,
    text: &str,
    fallback_package: &str,
    tests: &mut Vec<TestDraft>,
    sources: &mut Vec<SourceDraft>,
) {
    let mut offset = 0usize;
    for block in text.split("---\n") {
        let block_start = offset;
        offset += block.len() + "---\n".len();
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut lines = trimmed.lines();
        let Some(first_line) = lines.next() else {
            continue;
        };
        if let Some(("test", _)) = parse_annotation(first_line) {
            parse_test_block(relative, trimmed, block_start, fallback_package, tests);
        } else if let Some(("source", locator_value)) = parse_annotation(first_line) {
            parse_source_block(relative, trimmed, block_start, locator_value, sources);
        }
        // 未知の先頭行は自由記述として無視する（rust-cargo の
        // `@vtest.`以外の行と同じ扱い）。
    }
}

fn parse_test_block(
    relative: &str,
    block: &str,
    block_start: usize,
    fallback_package: &str,
    tests: &mut Vec<TestDraft>,
) {
    let mut id = None;
    let mut covers = Vec::new();
    let mut targets = Vec::new();
    let mut intent = None;
    for line in block.lines().skip(1) {
        let Some((key, value)) = parse_annotation(line) else {
            continue;
        };
        match key {
            "id" => id = Some(value.to_owned()),
            "covers" => covers.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(VoId::new),
            ),
            "target" => targets.push(value.to_owned()),
            "intent" => intent = Some(value.to_owned()),
            _ => {}
        }
    }
    let (Some(id), Some(intent)) = (id, intent) else {
        // 必須 metadata 欠落。この adapter は E-SCAN-005/006/007 相当の
        // 診断を独自に発行せず、core の Missing 経路（`missing_tests`）へ
        // も回さない — この adapter の唯一の目的（DS-1584 の確認）に
        // 必要な範囲を超える診断語彙をこの PR で新設しないための単純化。
        return;
    };
    if covers.is_empty() {
        return;
    }
    let location = SourceLocation {
        adapter: AdapterId::new(ADAPTER_ID),
        path: vtest_model::ProjectPath::new(relative),
        // DS-1584「Rust item pathではないopaque locator」: この locator は
        // `::` を一切含まない、ブロック先頭からの通し番号だけの opaque
        // 文字列。
        locator: format!("block-{block_start}"),
        byte_range: vtest_model::SourceRange {
            start: block_start as u64,
            end: (block_start + block.len()) as u64,
        },
    };
    tests.push(TestDraft {
        id: TestId::new(id),
        covers,
        targets: targets
            .into_iter()
            .map(|value| {
                TargetRef::Locator(Locator {
                    adapter: AdapterId::new(ADAPTER_ID),
                    value,
                })
            })
            .collect(),
        intent,
        input: None,
        expect: None,
        kind: None,
        cases: Vec::new(),
        related: Vec::new(),
        location: location.clone(),
        construct_text: block.to_owned(),
        execution: ExecutionDescriptor {
            adapter: AdapterId::new(ADAPTER_ID),
            project: Some(fallback_package.to_owned()),
            suite: None,
            selector: location.locator.clone(),
        },
    });
}

fn parse_source_block(
    relative: &str,
    block: &str,
    block_start: usize,
    locator_value: &str,
    sources: &mut Vec<SourceDraft>,
) {
    if locator_value.is_empty() {
        return;
    }
    sources.push(SourceDraft {
        locator: Locator {
            adapter: AdapterId::new(ADAPTER_ID),
            value: locator_value.to_owned(),
        },
        src_id: None,
        location: SourceLocation {
            adapter: AdapterId::new(ADAPTER_ID),
            path: vtest_model::ProjectPath::new(relative),
            locator: format!("block-{block_start}"),
            byte_range: vtest_model::SourceRange {
                start: block_start as u64,
                end: (block_start + block.len()) as u64,
            },
        },
        construct_text: block.to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @vtest.id TEST-ADAPTER-SYNTHETIC-DISCOVERS-NON-RUST-SOURCE
    /// @vtest.covers VO-ADAPTER-SYNTHETIC-DISCOVERS-NON-RUST-CONSTRUCT
    /// @vtest.target crates/vtest-adapter-synthetic/src/lib.rs::SyntheticAdapter::discover
    /// @vtest.intent verifies the synthetic adapter discovers a Test construct from a non-.rs file with a doc-comment-free metadata declaration and a colon-free opaque target locator
    #[test]
    fn discovers_a_non_rust_test_construct() {
        let root =
            std::env::temp_dir().join(format!("vtest-adapter-synthetic-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create fixture root");
        std::fs::write(
            root.join("fixture.synthetic"),
            "@synthetic.test\n@synthetic.id TEST-SYN-ONE\n@synthetic.covers VO-SYN-ONE\n@synthetic.target opaque-target-one\n@synthetic.intent a synthetic test construct\nbody text that is not a Rust function\n",
        )
        .expect("write fixture");

        let adapter = SyntheticAdapter::new();
        let outcome = adapter
            .discover(
                &root,
                "fallback",
                &AdapterScanConfig {
                    include_paths: vec![PathBuf::from(".")],
                },
            )
            .expect("discovery must succeed");

        assert_eq!(outcome.tests.len(), 1);
        let test = &outcome.tests[0];
        assert_eq!(test.id.as_str(), "TEST-SYN-ONE");
        assert_eq!(test.covers.len(), 1);
        assert_eq!(test.covers[0].as_str(), "VO-SYN-ONE");
        assert_eq!(test.targets.len(), 1);
        let TargetRef::Locator(locator) = &test.targets[0] else {
            panic!("expected a Locator target");
        };
        assert!(
            !locator.value.contains("::"),
            "the synthetic target locator must not use rust-cargo's `::` syntax"
        );
        assert!(!test.location.path.as_str().ends_with(".rs"));

        std::fs::remove_dir_all(&root).expect("remove fixture root");
    }
}
