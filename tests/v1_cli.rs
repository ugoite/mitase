use assert_cmd::Command;
use mitase_authoring::AuthoringDocument;
use mitase_spec_model::{BoundTargetRef, LocalAnchorKind, SpecDocument};
use mitase_workspace::SpecWorkspace;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};
use tempfile::tempdir;

#[test]
fn current_workspace_checks_and_reports_configured_readiness() {
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["check", "."])
        .assert()
        .success();
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["readiness", "report", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["target"], "traceable");
}

#[test]
fn first_run_short_authoring_fixture_passes_check() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/first-run-short");
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["check", fixture.to_str().unwrap()])
        .assert()
        .success();
}

#[derive(Debug, Deserialize)]
struct AcceptanceCorpus {
    source: AcceptanceSource,
    expected: AcceptanceExpected,
}

#[derive(Debug, Deserialize)]
struct AcceptanceSource {
    repository: String,
    revision: String,
    files: AcceptanceSourceFiles,
}

#[derive(Debug, Deserialize)]
struct AcceptanceSourceFiles {
    philosophies: String,
    policies: String,
}

#[derive(Debug, Deserialize)]
struct AcceptanceExpected {
    philosophy_count: usize,
    policy_count: usize,
    philosophy_to_policies: BTreeMap<String, Vec<String>>,
    policy_to_philosophies: BTreeMap<String, Vec<String>>,
}

#[test]
fn ugoite_foundation_policy_fixture_preserves_items_and_derived_governance() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/acceptance/ugoite-foundation-policy-v1");
    let temp = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());
    initialize_fixture_git(temp.path());

    let corpus: AcceptanceCorpus =
        serde_yaml::from_str(&fs::read_to_string(temp.path().join("corpus.yaml")).unwrap())
            .expect("acceptance corpus manifest");
    assert_eq!(corpus.source.repository, "ugoite/ugoite");
    assert_eq!(
        corpus.source.revision,
        "a872f4992bcb3633681eb0383e101453f00b32db"
    );
    assert_eq!(
        corpus.source.files.philosophies,
        "docs/spec/philosophy/foundation.yaml"
    );
    assert_eq!(
        corpus.source.files.policies,
        "docs/spec/policies/policies.yaml"
    );

    let workspace = SpecWorkspace::load(temp.path()).expect("acceptance workspace");
    let index = workspace.index().expect("acceptance index");
    let mut philosophy_ids = BTreeSet::new();
    let mut policy_ids = BTreeSet::new();
    for loaded in &workspace.documents {
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                for philosophy in philosophies {
                    assert_eq!(philosophy.principles.len(), 2);
                    assert!(
                        philosophy
                            .principles
                            .iter()
                            .any(|principle| principle.id.0 == "product-design-principle")
                    );
                    assert!(
                        philosophy
                            .principles
                            .iter()
                            .any(|principle| principle.id.0 == "coding-guideline")
                    );
                    philosophy_ids.insert(philosophy.id.0.clone());
                }
            }
            SpecDocument::Policies { policies, .. } => {
                for policy in policies {
                    assert_eq!(policy.rules.len(), 1);
                    assert_eq!(policy.rules[0].id.0, "governance");
                    assert!(!policy.rules[0].governed_by.is_empty());
                    assert!(policy.rules[0].governed_by.iter().all(|anchor| {
                        anchor.kind == LocalAnchorKind::Principle
                            && anchor.local_id.0 == "product-design-principle"
                    }));
                    policy_ids.insert(policy.id.0.clone());
                }
            }
            _ => {}
        }
    }
    assert_eq!(philosophy_ids.len(), corpus.expected.philosophy_count);
    assert_eq!(policy_ids.len(), corpus.expected.policy_count);
    assert_eq!(
        philosophy_ids,
        corpus
            .expected
            .philosophy_to_policies
            .keys()
            .cloned()
            .collect()
    );
    assert_eq!(
        policy_ids,
        corpus
            .expected
            .policy_to_philosophies
            .keys()
            .cloned()
            .collect()
    );

    let translated_philosophies =
        fs::read_to_string(temp.path().join("spec/foundation.yaml")).unwrap();
    let translated_policies = fs::read_to_string(temp.path().join("spec/policies.yaml")).unwrap();
    assert!(!translated_philosophies.contains("linked_policies:"));
    assert!(!translated_policies.contains("linked_philosophies:"));

    for (policy_id, expected_philosophies) in &corpus.expected.policy_to_philosophies {
        let rule = index
            .rules_to_principles
            .iter()
            .find(|(anchor, _)| {
                anchor.item.0.as_str() == policy_id.as_str()
                    && anchor.kind == LocalAnchorKind::Rule
                    && anchor.local_id.0.as_str() == "governance"
            })
            .map(|(_, principles)| principles)
            .expect("authored governance relation");
        let mut authored = rule
            .iter()
            .map(|anchor| anchor.item.0.clone())
            .collect::<Vec<_>>();
        authored.sort();
        let mut expected = expected_philosophies.clone();
        expected.sort();
        assert_eq!(authored, expected);
    }

    for (philosophy_id, expected_policies) in &corpus.expected.philosophy_to_policies {
        let principle = index
            .principles_to_rules
            .keys()
            .find(|anchor| {
                anchor.item.0.as_str() == philosophy_id.as_str()
                    && anchor.kind == LocalAnchorKind::Principle
                    && anchor.local_id.0.as_str() == "product-design-principle"
            })
            .expect("derived principle anchor");
        let mut derived = index
            .principles_to_rules
            .get(principle)
            .expect("derived governance relation")
            .iter()
            .map(|anchor| anchor.item.0.clone())
            .collect::<Vec<_>>();
        derived.sort();
        let mut expected = expected_policies.clone();
        expected.sort();
        assert_eq!(derived, expected);
    }

    Command::cargo_bin("mitase")
        .unwrap()
        .args(["check", "."])
        .current_dir(temp.path())
        .assert()
        .success();
}

#[test]
fn public_cli_does_not_expose_work_or_task_commands() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("work "))
    );
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("task "))
    );
}

#[test]
fn public_cli_does_not_expose_transitional_validation_commands() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("plan "))
    );
    assert!(
        !help
            .lines()
            .any(|line| line.trim_start().starts_with("result "))
    );
}

#[test]
fn show_and_list_expose_deterministic_semantic_read_models() {
    let first = Command::cargo_bin("mitase")
        .unwrap()
        .args(["list", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(first.status.success());
    let second = Command::cargo_bin("mitase")
        .unwrap()
        .args(["list", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(second.status.success());
    assert_eq!(first.stdout, second.stdout);

    let list: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(list["items"][0]["kind"], "feature");
    assert_eq!(list["items"][0]["id"], "FEAT-CHANGE-VALIDATION-001");
    assert_eq!(list["items"][0]["namespace"], "capabilities");
    assert_eq!(list["items"][0]["category"], "Mitase functional units");

    let filtered = Command::cargo_bin("mitase")
        .unwrap()
        .args([
            "list",
            ".",
            "--kind",
            "requirement",
            "--status",
            "implemented",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(filtered.status.success());
    let filtered: serde_json::Value = serde_json::from_slice(&filtered.stdout).unwrap();
    assert!(
        filtered["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| { item["kind"] == "requirement" && item["status"] == "implemented" })
    );

    let show = Command::cargo_bin("mitase")
        .unwrap()
        .args(["show", "REQ-CAPABILITY-001", ".", "--format", "json"])
        .output()
        .unwrap();
    assert!(show.status.success());
    let show: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(show["id"], "REQ-CAPABILITY-001");
    assert_eq!(show["kind"], "requirement");
    assert!(
        show["authored_relations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|relation| { relation["relation"] == "governed-by" })
    );
    assert!(
        show["derived_relations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|relation| { relation["relation"] == "implementation-targets" })
    );
    assert!(
        show["criteria"]
            .as_array()
            .unwrap()
            .iter()
            .any(|criterion| {
                criterion["id"] == "REQ-CAPABILITY-001#criterion.spec-model"
                    && criterion["implementation_targets"]
                        .as_array()
                        .is_some_and(|targets| !targets.is_empty())
                    && criterion["verification_targets"]
                        .as_array()
                        .is_some_and(|targets| !targets.is_empty())
                    && criterion["verification"] == "verified"
            })
    );
    assert!(
        show["verification_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|claim| { claim["assessment"]["status"] == "valid" })
    );
}

#[test]
fn list_filters_canonical_metadata_and_reports_unverified_criteria() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/mitase")).unwrap();
    fs::write(
        temp.path().join("mitase.yaml"),
        "schema: mitase/config/v1\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("docs/mitase/requirements.yaml"),
        r#"schema: mitase/spec/v1
kind: requirements
namespace: demo
category: Demo requirements
requirements:
- id: REQ-DEMO-001
  title: An inspectable requirement
  description: A requirement with intentionally incomplete evidence.
  priority: high
  status: implemented
  criteria:
  - id: acceptance
    kind: behavior
    statement: The requirement is visible through the read model.
    governed_by: []
"#,
    )
    .unwrap();

    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args([
            "list",
            "--namespace",
            "demo",
            "--category",
            "Demo requirements",
            "--unverified-criteria",
            "--format",
            "json",
        ])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["items"][0]["namespace"], "demo");
    assert_eq!(result["items"][0]["category"], "Demo requirements");
    assert_eq!(result["unverified_criteria"].as_array().unwrap().len(), 1);
    assert_eq!(
        result["unverified_criteria"][0]["id"],
        "REQ-DEMO-001#criterion.acceptance"
    );
    assert_eq!(
        result["unverified_criteria"][0]["verification"],
        "unverified"
    );

    let show = Command::cargo_bin("mitase")
        .unwrap()
        .args(["show", "REQ-DEMO-001", "--format", "json"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(
        show.status.success(),
        "{}",
        String::from_utf8_lossy(&show.stderr)
    );
    let show: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(show["criteria"][0]["verification"], "unverified");
}

#[test]
fn show_does_not_mark_invalid_runner_metadata_as_verified() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/v1/valid-web-app");
    let temp = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());
    let config_path = temp.path().join("mitase.yaml");
    let config = fs::read_to_string(&config_path).unwrap();
    let config = config
        .replace(
            "verification:\n  runners:\n    cargo-test:\n      executable: cargo\n      arguments: [test, -p, \"{package}\", \"{test}\"]\n",
            "verification:\n  runners: {}\n",
        );
    fs::write(&config_path, config).unwrap();

    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["show", "REQ-AUTH-001", "--format", "json"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let show: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(show["criteria"][0]["verification"], "unverified");
}

#[test]
fn query_exposes_explicit_canonical_relations() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args([
            "query",
            "REQ-CAPABILITY-001#criterion.spec-model",
            ".",
            "--relation",
            "implementation-targets",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["source"], "REQ-CAPABILITY-001#criterion.spec-model");
    assert_eq!(result["relations"][0]["relation"], "implementation-targets");
}

#[test]
fn query_reports_unknown_sources_with_nonzero_exit() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["query", "REQ-DOES-NOT-EXIST"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("query source"));
}

#[test]
fn migrate_is_explicit_read_only_and_preserves_the_canonical_document() {
    let temp = tempdir().unwrap();
    let source_path = temp.path().join("requirement.yaml");
    let source = r#"
schema: mitase/spec/v1
kind: requirements
namespace: demo
category: Demo
requirements: []
"#;
    fs::write(&source_path, source).unwrap();

    let without_stdout = Command::cargo_bin("mitase")
        .unwrap()
        .args(["migrate"])
        .arg(&source_path)
        .output()
        .unwrap();
    assert!(!without_stdout.status.success());
    assert!(String::from_utf8_lossy(&without_stdout.stderr).contains("--stdout"));
    assert_eq!(fs::read_to_string(&source_path).unwrap(), source);

    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["migrate"])
        .arg(&source_path)
        .arg("--stdout")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(&source_path).unwrap(), source);
    let migrated = AuthoringDocument::parse(&String::from_utf8(output.stdout).unwrap())
        .expect("v0.2 migration output");
    let canonical: SpecDocument = serde_yaml::from_str(source).unwrap();
    assert_eq!(migrated.normalize().unwrap().document, canonical);
}

#[test]
fn config_effective_reports_resolved_conventions_without_loading_or_mutating_specs() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("mitase.yaml"),
        "schema: mitase/config/v1\n",
    )
    .unwrap();

    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["config", "effective"])
        .arg(temp.path())
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], "mitase/config/v1");
    assert_eq!(report["workspace"]["spec_roots"][0], "docs/mitase");
    assert_eq!(report["inventory"]["active_profile"], "default");
    assert!(
        report["applied_conventions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|convention| convention == "workspace.spec_roots=docs/mitase")
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("mitase.yaml")).unwrap(),
        "schema: mitase/config/v1\n"
    );
}

#[test]
fn frontend_load_diagnostics_use_the_validation_shape_for_cli_json_and_text() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/mitase")).unwrap();
    fs::write(
        temp.path().join("mitase.yaml"),
        "schema: mitase/config/v1\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("docs/mitase/invalid.yaml"),
        concat!(
            "schema: mitase/authoring/v2\n",
            "kind: requirement\n",
            "namespace: test\n",
            "category: Test\n",
            "requirement:\n",
            "  id: REQ-CLI-INVALID-001\n",
            "  title: Invalid frontend input\n",
            "  description: The adapter cannot be inferred.\n",
            "  priority: medium\n",
            "  status: planned\n",
            "  criterion: { id: behavior, kind: behavior, statement: Explicit, governed_by: [] }\n",
            "  implementation:\n",
            "    facet: delivery\n",
            "    responsibility: Own the implementation.\n",
            "    target: { path: src/example.txt, satisfies: behavior }\n",
            "  verification:\n",
            "    facet: verification\n",
            "    responsibility: Verify the implementation.\n",
            "    target:\n",
            "      adapter: rust\n",
            "      path: src/lib.rs\n",
            "      verifies: { criterion: behavior, covers: [source], runner: cargo-test }\n",
        ),
    )
    .unwrap();
    initialize_fixture_git(temp.path());

    let json = Command::cargo_bin("mitase")
        .unwrap()
        .args(["check", "--format", "json"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!json.status.success());
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let diagnostic = &report["diagnostics"][0];
    assert_eq!(diagnostic["code"], "MITASE-AUTHORING-002");
    assert!(diagnostic["primary"]["line"].is_number());
    assert!(diagnostic["reason"].is_string());
    assert!(diagnostic["suggested_action"].is_string());

    let text = Command::cargo_bin("mitase")
        .unwrap()
        .args(["check", "--format", "text"])
        .arg(temp.path())
        .output()
        .unwrap();
    let rendered = String::from_utf8_lossy(&text.stderr);
    assert!(rendered.contains("MITASE-AUTHORING-002"));
    assert!(rendered.contains("suggested action"));
}

#[test]
fn tutorial_yaml_examples_parse_and_normalize_as_v2_authoring_documents() {
    let tutorial = fs::read_to_string("docs/start-here/first-run/tutorial.md").expect("tutorial");
    let mut blocks = Vec::new();
    let mut in_yaml = false;
    let mut current = String::new();
    for line in tutorial.lines() {
        if line.trim() == "```yaml" {
            assert!(!in_yaml, "nested YAML code block");
            in_yaml = true;
            current.clear();
        } else if in_yaml && line.trim() == "```" {
            blocks.push(current.clone());
            in_yaml = false;
        } else if in_yaml {
            current.push_str(line);
            current.push('\n');
        }
    }
    assert!(!in_yaml, "unterminated YAML code block");
    assert_eq!(blocks.len(), 1);
    for block in blocks {
        let authoring = AuthoringDocument::parse(&block).expect("tutorial YAML must be v2");
        authoring
            .normalize()
            .expect("tutorial v2 YAML must normalize");
    }
}

#[test]
fn show_reports_missing_specifications_with_nonzero_exit() {
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["show", "REQ-DOES-NOT-EXIST"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("was not found"));
}

#[test]
fn generated_spec_reference_covers_every_source_document() {
    let index =
        fs::read_to_string("docs/reference/specification/index.md").expect("generated index");
    let mut sources = Vec::new();
    collect_spec_yaml_files(Path::new("docs/mitase"), &mut sources);
    for source in sources {
        let relative = source
            .strip_prefix("docs/mitase")
            .expect("spec source path");
        let generated =
            Path::new("docs/reference/specification").join(generated_spec_path(relative));
        let page = fs::read_to_string(&generated)
            .unwrap_or_else(|error| panic!("read {}: {error}", generated.display()));
        let source_display = source.to_string_lossy();
        assert!(
            page.contains(&format!("Generated from `{source_display}`")),
            "{} does not identify its canonical source",
            generated.display()
        );
        let doc_link = generated_spec_path(relative)
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        assert!(
            index.contains(&format!("({doc_link}.md)")),
            "generated index does not link {doc_link}"
        );
    }
}

#[test]
fn mitase_authoring_v2_preserves_the_pre_migration_canonical_graph() {
    let workspace = SpecWorkspace::load(".").expect("Mitase workspace");
    let expected = fs::read_to_string("tests/fixtures/mitase-v1-canonical-digests.txt")
        .expect("canonical graph baseline")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (digest, path) = line.split_once("  ").expect("digest fixture entry");
            (path.to_owned(), digest.to_owned())
        })
        .collect::<BTreeMap<_, _>>();

    let actual = workspace
        .documents
        .iter()
        .map(|loaded| {
            let relative = loaded
                .path
                .strip_prefix(&workspace.root)
                .expect("document under workspace root")
                .to_string_lossy()
                .into_owned();
            let canonical = serde_yaml::to_string(&loaded.document).expect("canonical document");
            let mut hasher = Sha256::new();
            hasher.update(canonical.as_bytes());
            let digest = hasher.finalize();
            let digest = digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            (relative, digest)
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(actual, expected);
}

#[test]
fn self_hosted_config_preserves_the_exact_artifact_resolution_baseline() {
    let workspace = SpecWorkspace::load(".").expect("Mitase workspace");
    let index = workspace.index().expect("Mitase index");
    let expected = fs::read_to_string("tests/fixtures/mitase-artifact-resolution-baseline.txt")
        .expect("artifact resolution baseline")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (reference, artifact) = line
                .split_once("  ")
                .expect("artifact resolution baseline entry");
            let reference: BoundTargetRef = reference.parse().expect("target reference");
            (reference.to_string(), artifact.to_owned())
        })
        .collect::<BTreeMap<_, _>>();
    let actual = index
        .target_to_artifact
        .iter()
        .map(|(reference, artifact)| (reference.to_string(), artifact.clone()))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(actual, expected);
}

#[test]
fn mitase_authoring_corpus_measurement_matches_the_short_contract_boundary() {
    let workspace = SpecWorkspace::load(".").expect("Mitase workspace");
    assert_eq!(workspace.documents.len(), 11);

    let mut philosophy_count = 0;
    let mut policy_count = 0;
    let mut requirement_count = 0;
    let mut feature_count = 0;
    let mut requirement_shapes = Vec::new();

    for loaded in &workspace.documents {
        match &loaded.document {
            SpecDocument::Philosophies { philosophies, .. } => {
                philosophy_count += philosophies.len();
            }
            SpecDocument::Policies { policies, .. } => {
                policy_count += policies.len();
            }
            SpecDocument::Requirements { requirements, .. } => {
                requirement_count += requirements.len();
                requirement_shapes.extend(
                    requirements.iter().map(|requirement| {
                        (requirement.criteria.len(), requirement.bindings.len())
                    }),
                );
            }
            SpecDocument::Features { features, .. } => {
                feature_count += features.len();
            }
        }
    }

    assert_eq!(
        (
            philosophy_count,
            policy_count,
            requirement_count,
            feature_count
        ),
        (3, 7, 2, 18)
    );
    requirement_shapes.sort_unstable();
    assert_eq!(requirement_shapes, vec![(3, 1), (13, 2)]);
}

fn generated_spec_path(relative: &Path) -> PathBuf {
    let parts: Vec<_> = relative.components().collect();
    let section = match parts.as_slice() {
        [first, ..]
            if first.as_os_str() == "philosophies"
                || first.as_os_str() == "policies"
                || first.as_os_str() == "requirements" =>
        {
            "foundations"
        }
        [first, second, ..]
            if first.as_os_str() == "features" && second.as_os_str() == "public-entrypoints" =>
        {
            "contracts"
        }
        [first, ..] if first.as_os_str() == "features" => "capabilities",
        _ => panic!("unsupported specification path: {}", relative.display()),
    };
    PathBuf::from(section)
        .join(relative.file_name().expect("spec filename"))
        .with_extension("md")
}

fn collect_spec_yaml_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("spec directory") {
        let entry = entry.expect("spec entry");
        let path = entry.path();
        if path.is_dir() {
            collect_spec_yaml_files(&path, files);
        } else if path.extension().and_then(|value| value.to_str()) == Some("yaml") {
            files.push(path);
        }
    }
}

#[test]
fn obsolete_config_shape_is_rejected() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs/mitase")).unwrap();
    fs::write(
        temp.path().join("mitase.yaml"),
        "schema: mitase/config/v1\nversion: 1\nspec: { root: docs/mitase }\n",
    )
    .unwrap();
    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("obsolete pre-release"));
}

#[test]
fn obsolete_pre_release_fixture_is_rejected_under_canonical_mitase_identity() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/rejected/obsolete-pre-release-v1");
    let temp = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());

    let config = fs::read_to_string(temp.path().join("mitase.yaml")).unwrap();
    assert!(config.contains("schema: mitase/config/v1"));

    let output = Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("obsolete pre-release"));
}

fn copy_fixture_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_fixture_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).unwrap();
        }
    }
}

fn initialize_fixture_git(root: &Path) {
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "mitase-tests@example.invalid"],
        vec!["config", "user.name", "Mitase Tests"],
        vec!["add", "."],
        vec!["commit", "-qm", "fixture baseline"],
    ] {
        assert!(
            ProcessCommand::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
}

fn staged_validation_fixture() -> tempfile::TempDir {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/v1/valid-web-app");
    let temp = tempdir().unwrap();
    copy_fixture_tree(&fixture, temp.path());
    let config_path = temp.path().join("mitase.yaml");
    let config = fs::read_to_string(&config_path)
        .unwrap()
        .replace(
            "require_owned_changes: true",
            "require_owned_changes: false",
        )
        .replace("require_plan: true", "require_plan: false");
    fs::write(config_path, config).unwrap();
    initialize_fixture_git(temp.path());
    temp
}

#[test]
fn staged_change_validation_uses_the_index_snapshot() {
    let temp = staged_validation_fixture();
    let feature = temp.path().join("spec/feature.yaml");
    fs::write(
        &feature,
        format!("{}\n", fs::read_to_string(&feature).unwrap()),
    )
    .unwrap();
    assert!(
        ProcessCommand::new("git")
            .args(["add", "spec/feature.yaml"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success()
    );

    fs::write(temp.path().join("mitase.yaml"), "not: [valid\n").unwrap();
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .success();
}

#[test]
fn staged_change_validation_rejects_invalid_index_content_and_invalid_options() {
    let temp = staged_validation_fixture();
    let config_path = temp.path().join("mitase.yaml");
    let original = fs::read_to_string(&config_path).unwrap();
    fs::write(&config_path, "not: [valid\n").unwrap();
    assert!(
        ProcessCommand::new("git")
            .args(["add", "mitase.yaml"])
            .current_dir(temp.path())
            .status()
            .unwrap()
            .success()
    );
    fs::write(&config_path, original).unwrap();

    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .failure();
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "workspace"])
        .arg(temp.path())
        .arg("--staged")
        .assert()
        .failure();
    Command::cargo_bin("mitase")
        .unwrap()
        .args(["validate", "change"])
        .arg(temp.path())
        .args(["--staged", "--baseline", "parent"])
        .assert()
        .failure();
}
