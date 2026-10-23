//! Process-boundary and byte-identity tests for the local producer operator
//! commands (Build/Qualification M009).
//!
//! The documented stdout/stderr contract, the exit discipline, and the
//! byte-identity of the local plan and release artifacts against the existing
//! `ci _*` runner family are only observable from outside the process, so they
//! are asserted here.
//!
//! These tests are hermetic: they read fixtures checked into this repository
//! plus temporary directories. No network, no repository discovery, no release
//! selection, no staging, and no publication.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::Digest;

/// Canonical contract fixture used by the validation-surface tests.
const SIMPLE_CONTRACT: &str =
    include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");

/// Bounded GitHub draft template the `ci _resolve-release` runner requires.
const TEMPLATE: &str = r#"{
  "schema_version": 1,
  "owner": "acme",
  "repository": "widget",
  "title_prefix": "widget ",
  "body": "notes",
  "prerelease": false,
  "token_env": "GITHUB_TOKEN",
  "request_timeout_secs": 30,
  "max_metadata_bytes": 1000000,
  "max_list_pages": 5
}"#;

/// Bounded provider policy the `ci generate` and `ci check` commands consume.
const GITHUB_POLICY: &str = r#"{
  "preflight_runner": "ubuntu-latest",
  "runners": [
    {
      "os": "linux",
      "arch": "x86_64",
      "label": "ubuntu-latest",
      "cargo_zigbuild": false,
      "zig": false
    }
  ],
  "checkout": {"reference": "actions/checkout@0123456789abcdef0123456789abcdef01234567"},
  "rust_toolchain": {"reference": "dtolnay/rust-toolchain@0123456789abcdef0123456789abcdef01234567"},
  "upload_artifact": {"reference": "actions/upload-artifact@0123456789abcdef0123456789abcdef01234567"},
  "download_artifact": {"reference": "actions/download-artifact@0123456789abcdef0123456789abcdef01234567"},
  "triggers": ["push", "workflow_dispatch"],
  "timeout_minutes": 60,
  "cancel_in_progress": true,
  "artifact_retention_days": 7,
  "eggpack_tool": {
    "repo": "https://github.com/eggstack/eggpack",
    "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "package": "eggpack-cli",
    "install_timeout_minutes": 10
  },
  "release_inputs": {
    "contract": "contracts/release.toml",
    "release_plan": "plans/release-plan.json",
    "build_bindings": "bindings/build.toml",
    "qualification_bindings": "bindings/qualification.toml",
    "ci_plan": "plans/release-ci-plan.json"
  }
}"#;

/// Canonical target triple used by the portable (no real build) tests.
const TRIPLE: &str = "x86_64-unknown-linux-gnu";
/// Canonical alias of [`TRIPLE`].
const ALIAS: &str = "linux-x64";

/// Release identity shared by the local path and the runner family.
const RELEASE_ID: &str = "v1.2.3";

/// A syntactically valid, non-existent source revision.
const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> PathBuf {
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "eggpack-cli-operator-{label}-{}-{id}",
            std::process::id()
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create test temp directory: {error}"),
        }
    }
}

struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

fn eggpack(args: &[String]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_eggpack"))
        .args(args)
        .output()
        .expect("run eggpack binary");
    Output {
        code: output.status.code().expect("process exited normally"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn write_file(path: &Path, contents: &str) -> String {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

/// One release's producer input set.
struct Inputs {
    /// Contract TOML path.
    contract: String,
    /// Producer policy TOML path.
    pack_config: String,
    /// Build bindings TOML path.
    build_bindings: String,
    /// Qualification bindings TOML path.
    qualification_bindings: String,
    /// GitHub draft template JSON path.
    template: String,
}

/// Write the canonical producer input set for one target.
fn write_inputs(root: &Path, triple: &str, alias: &str, host_os: &str, host_arch: &str) -> Inputs {
    let contract = write_file(
        &root.join("contract.toml"),
        &format!(
            "schema_version = 1\n\n[product]\nid = \"eggsact\"\ndisplay_name = \"eggsact\"\n\n[[targets]]\ntriple = \"{triple}\"\naliases = [\"{alias}\"]\n\n[targets.asset]\nkind = \"direct\"\nasset = \"{{product}}-{{version}}-{{target}}\"\ninstall = \"{{product}}\"\n\n[targets.checksum]\nsidecar = \"{{asset}}.sha256\"\n"
        ),
    );
    let pack_config = write_file(
        &root.join("pack.toml"),
        &format!(
            "schema_version = 1\n\n[[targets]]\ntarget = \"{triple}\"\nstrategy = \"native_cargo\"\nhost_os = \"{host_os}\"\nhost_arch = \"{host_arch}\"\nqualification = \"structural\"\nsupport = \"required\"\n\n[targets.toolchain]\nrust = \"stable\"\n\n[targets.floor]\nkind = \"none\"\n"
        ),
    );
    let build_bindings = write_file(
        &root.join("build.toml"),
        &format!(
            "schema_version = 1\n\n[[targets.\"{triple}\"]]\nselector = {{ kind = \"direct\" }}\npackage = \"eggsact\"\nbinary = \"bin0\"\n"
        ),
    );
    let qualification_bindings = write_file(
        &root.join("qualification.toml"),
        &format!("schema_version = 1\n\n[targets.\"{triple}\"]\n"),
    );
    let template = write_file(&root.join("template.json"), TEMPLATE);
    Inputs {
        contract,
        pack_config,
        build_bindings,
        qualification_bindings,
        template,
    }
}

/// Initialize a git repository with one commit and one exact tag.
fn init_git_repo(root: &Path, tag: &str) -> String {
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "Eggpack test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Eggpack test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {args:?}");
    };
    run(&["init", "-q"]);
    std::fs::write(root.join("source.txt"), "revision").unwrap();
    run(&["add", "source.txt"]);
    run(&["commit", "-q", "-m", "revision"]);
    run(&["tag", tag]);
    let output = Command::new("git")
        .args(["rev-parse", "HEAD^{commit}"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// Write a Cargo project that builds one binary named `bin0`.
fn write_cargo_fixture(root: &Path) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"eggsact\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"bin0\"\npath = \"src/main.rs\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Cargo.lock"),
        "version = 3\n\n[[package]]\nname = \"eggsact\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
}

/// The host target triple, alias, operating system, and architecture.
fn host_target() -> (String, String, &'static str, &'static str) {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => (
            "x86_64-pc-windows-msvc".to_owned(),
            "windows-x64".to_owned(),
            "windows",
            "x86_64",
        ),
        ("macos", "x86_64") => (
            "x86_64-apple-darwin".to_owned(),
            "macos-x64".to_owned(),
            "macos",
            "x86_64",
        ),
        ("macos", "aarch64") => (
            "aarch64-apple-darwin".to_owned(),
            "macos-arm64".to_owned(),
            "macos",
            "aarch64",
        ),
        ("linux", "x86_64") => (
            "x86_64-unknown-linux-gnu".to_owned(),
            "linux-x64".to_owned(),
            "linux",
            "x86_64",
        ),
        ("linux", "aarch64") => (
            "aarch64-unknown-linux-gnu".to_owned(),
            "linux-arm64".to_owned(),
            "linux",
            "aarch64",
        ),
        other => panic!("unsupported host platform: {other:?}"),
    }
}

/// The valid argument vector of one command, with placeholder paths.
fn valid_args(command: &str) -> Vec<String> {
    match command {
        "check" => argv(&[
            command,
            "--contract",
            "contract.toml",
            "--pack-config",
            "pack.toml",
            "--build-bindings",
            "build.toml",
            "--qualification-bindings",
            "qualification.toml",
            "--selected",
            ALIAS,
            "--release-id",
            RELEASE_ID,
            "--source-revision",
            REVISION,
        ]),
        "plan" => argv(&[
            command,
            "--contract",
            "contract.toml",
            "--pack-config",
            "pack.toml",
            "--build-bindings",
            "build.toml",
            "--qualification-bindings",
            "qualification.toml",
            "--selected",
            ALIAS,
            "--release-id",
            RELEASE_ID,
            "--source-revision",
            REVISION,
            "--output",
            "release-plan.json",
        ]),
        "package" => argv(&[
            command,
            "--release-plan",
            "release-plan.json",
            "--build-bindings",
            "build.toml",
            "--source-root",
            "source",
            "--output-root",
            "out",
        ]),
        "qualify" => argv(&[
            command,
            "--contract",
            "contract.toml",
            "--release-plan",
            "release-plan.json",
            "--build-bindings",
            "build.toml",
            "--qualification-bindings",
            "qualification.toml",
            "--artifacts-dir",
            "candidates",
            "--output-dir",
            "qualified",
        ]),
        "collect" => argv(&[
            command,
            "--contract",
            "contract.toml",
            "--release-plan",
            "release-plan.json",
            "--build-bindings",
            "build.toml",
            "--qualification-bindings",
            "qualification.toml",
            "--artifacts-dir",
            "candidates",
            "--output-root",
            "finalized",
        ]),
        "manifest" => argv(&[
            command,
            "--contract",
            "contract.toml",
            "--release-plan",
            "release-plan.json",
            "--build-bindings",
            "build.toml",
            "--qualification-bindings",
            "qualification.toml",
            "--finalized-root",
            "finalized",
            "--output",
            "release-manifest.json",
        ]),
        other => panic!("unknown command: {other}"),
    }
}

/// Every command in the operator family.
const COMMANDS: [&str; 6] = ["check", "plan", "package", "qualify", "collect", "manifest"];

/// Rewrite `--flag value` pairs into the `--flag=value` form.
fn to_equals_form(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg.starts_with("--") && index + 1 < args.len() && !args[index + 1].starts_with("--") {
            out.push(format!("{arg}={}", args[index + 1]));
            index += 2;
        } else {
            out.push(arg.clone());
            index += 1;
        }
    }
    out
}

/// Assert a failing invocation: nonzero exit, one bounded diagnostic, and no
/// stdout at all.
fn assert_failure(out: &Output, context: &str) {
    assert_eq!(out.code, 1, "{context} must exit with code one");
    assert_eq!(out.stdout, "", "{context} must not write to stdout");
    assert!(
        out.stderr.starts_with("eggpack: "),
        "{context} diagnostic: {}",
        out.stderr
    );
    assert_eq!(
        out.stderr.lines().count(),
        1,
        "{context} must emit exactly one diagnostic line"
    );
    assert!(
        out.stderr.len() < 1024,
        "{context} diagnostic must stay bounded"
    );
}

/// Every command exits zero on success and one on failure.
#[test]
fn every_command_exits_zero_on_success_and_one_on_failure() {
    let root = temp_root("exit-codes");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    for command in COMMANDS {
        let mut args = valid_args(command);
        // Point the first document flag at a document that does not exist.
        let index = args
            .iter()
            .position(|value| value == "--contract")
            .or_else(|| args.iter().position(|value| value == "--release-plan"))
            .expect("a document flag");
        args[index + 1] = root.join("absent.json").to_string_lossy().into_owned();
        let out = eggpack(&args);
        assert_eq!(out.code, 1, "{command} must exit with code one");
        assert_eq!(out.stdout, "", "{command} must not write to stdout");
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` accepts a valid input set and reports one verdict line.
#[test]
fn check_accepts_a_valid_input_set() {
    let root = temp_root("check-ok");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "check: ok\n");
    assert_eq!(out.stderr, "");
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` refuses a contract with an unknown field.
#[test]
fn check_rejects_a_contract_with_an_unknown_field() {
    let root = temp_root("check-unknown-field");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let broken = write_file(
        &root.join("broken.toml"),
        &format!("{}\nunknown_field = 1\n", SIMPLE_CONTRACT),
    );
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &broken,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_failure(&out, "unknown contract field");
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` refuses a contract with the wrong schema version.
#[test]
fn check_rejects_a_contract_with_the_wrong_schema_version() {
    let root = temp_root("check-schema-version");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let broken = write_file(
        &root.join("broken.toml"),
        &SIMPLE_CONTRACT.replacen("schema_version = 1", "schema_version = 2", 1),
    );
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &broken,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_failure(&out, "wrong contract schema version");
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` refuses a selected target the producer policy does not
/// cover.
#[test]
fn check_rejects_an_uncovered_selected_target() {
    let root = temp_root("check-uncovered");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        "macos-arm64",
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_failure(&out, "uncovered selected target");
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` refuses a binding for a target outside the release plan.
#[test]
fn check_rejects_an_extra_build_binding() {
    let root = temp_root("check-extra-binding");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let extra = write_file(
        &root.join("extra.toml"),
        &format!(
            "schema_version = 1\n\n[[targets.\"{TRIPLE}\"]]\nselector = {{ kind = \"direct\" }}\npackage = \"eggsact\"\nbinary = \"bin0\"\n\n[[targets.\"aarch64-apple-darwin\"]]\nselector = {{ kind = \"direct\" }}\npackage = \"eggsact\"\nbinary = \"bin0\"\n"
        ),
    );
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &extra,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_failure(&out, "extra build binding");
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack plan` writes the same bytes as the `ci _resolve-release` runner.
#[test]
fn plan_writes_the_same_bytes_as_ci_resolve_release() {
    let root = temp_root("plan-bytes");
    let revision = init_git_repo(&root, RELEASE_ID);
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let runner_plan = root.join("runner-plan.json");
    let out = eggpack(&argv(&[
        "ci",
        "_resolve-release",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--tag",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--template",
        &inputs.template,
        "--source-root",
        &root.to_string_lossy(),
        "--output-plan",
        &runner_plan.to_string_lossy(),
        "--output-ci-plan",
        &root.join("runner-ci-plan.json").to_string_lossy(),
        "--output-github-policy",
        &root.join("runner-github.json").to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);

    let local_plan = root.join("local-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--output",
        &local_plan.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "plan: ok\n");
    assert_eq!(
        std::fs::read(&local_plan).unwrap(),
        std::fs::read(&runner_plan).unwrap(),
        "eggpack plan and ci _resolve-release must write identical plan bytes"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// The local build → qualify → collect → manifest chain produces the same
/// release manifest bytes as the `ci _aggregate` runner, and the same bytes as
/// the manifest derived from the finalized tree.
#[test]
fn package_qualify_collect_manifest_matches_ci_aggregate() {
    let root = temp_root("chain");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);

    // The release plan the whole chain consumes.
    let plan_path = root.join("release-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        &alias,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--output",
        &plan_path.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "plan stderr: {}", out.stderr);
    assert_eq!(out.stdout, "plan: ok\n");

    // Build every planned target under the caller's output root.
    let candidates = root.join("candidates");
    let out = eggpack(&argv(&[
        "package",
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--source-root",
        &source.to_string_lossy(),
        "--output-root",
        &candidates.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "package stderr: {}", out.stderr);
    assert_eq!(out.stdout, "package: ok\n");
    assert!(candidates.join(&triple).join("build-handoff.json").exists());

    // Qualify the built candidates.
    let qualified = root.join("qualified");
    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &qualified.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "qualify stderr: {}", out.stderr);
    assert_eq!(out.stdout, "qualify: ok\n");
    assert!(qualified.join(&triple).join("evidence.json").exists());

    // Aggregate the qualified candidates into a finalized release tree.
    let local = root.join("local");
    std::fs::create_dir_all(&local).unwrap();
    let finalized = local.join("finalized");
    let out = eggpack(&argv(&[
        "collect",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &finalized.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "collect stderr: {}", out.stderr);
    assert_eq!(out.stdout, "collect: ok\n");
    let collected_manifest = local.join("release-manifest.json");
    assert!(collected_manifest.exists());

    // The manifest derived from the finalized tree is the same document.
    let derived_manifest = root.join("derived-manifest.json");
    let out = eggpack(&argv(&[
        "manifest",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--finalized-root",
        &finalized.to_string_lossy(),
        "--output",
        &derived_manifest.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "manifest stderr: {}", out.stderr);
    assert_eq!(out.stdout, "manifest: ok\n");
    assert_eq!(
        std::fs::read(&derived_manifest).unwrap(),
        std::fs::read(&collected_manifest).unwrap(),
        "the derived manifest must equal the aggregated manifest"
    );

    // The runner family aggregates the same bytes into the same manifest.
    let ci_plan = root.join("ci-plan.json");
    let out = eggpack(&argv(&[
        "ci",
        "_resolve-release",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        &alias,
        "--tag",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--template",
        &inputs.template,
        "--source-root",
        &repo.to_string_lossy(),
        "--output-plan",
        &root.join("runner-plan.json").to_string_lossy(),
        "--output-ci-plan",
        &ci_plan.to_string_lossy(),
        "--output-github-policy",
        &root.join("runner-github.json").to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "resolve stderr: {}", out.stderr);
    let runner = root.join("runner");
    std::fs::create_dir_all(&runner).unwrap();
    let runner_finalized = runner.join("finalized");
    let out = eggpack(&argv(&[
        "ci",
        "_aggregate",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--ci-plan",
        &ci_plan.to_string_lossy(),
        "--inputs-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &runner_finalized.to_string_lossy(),
        "--output",
        &runner.join("summary.json").to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "aggregate stderr: {}", out.stderr);
    assert_eq!(
        std::fs::read(runner.join("release-manifest.json")).unwrap(),
        std::fs::read(&collected_manifest).unwrap(),
        "eggpack collect and ci _aggregate must write identical manifest bytes"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// Every command refuses an unknown flag.
#[test]
fn every_command_rejects_an_unknown_flag() {
    for command in COMMANDS {
        let mut args = valid_args(command);
        args.push("--not-a-flag".to_owned());
        args.push("value".to_owned());
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} unknown flag"));
        assert!(
            out.stderr.contains("unknown option"),
            "{command} diagnostic: {}",
            out.stderr
        );
    }
}

/// Every command refuses a duplicated flag.
#[test]
fn every_command_rejects_a_duplicate_flag() {
    for command in COMMANDS {
        let mut args = valid_args(command);
        let first = args[1].clone();
        args.push(first);
        args.push("value".to_owned());
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} duplicate flag"));
        assert!(
            out.stderr.contains("only once"),
            "{command} diagnostic: {}",
            out.stderr
        );
    }
}

/// Every command refuses an empty flag value.
#[test]
fn every_command_rejects_an_empty_value() {
    for command in COMMANDS {
        let mut args = valid_args(command);
        let index = args
            .iter()
            .position(|value| value == "--contract")
            .or_else(|| args.iter().position(|value| value == "--release-plan"))
            .expect("a value-bearing flag");
        args[index + 1] = String::new();
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} empty value"));
        assert!(
            out.stderr.contains("must not be empty"),
            "{command} diagnostic: {}",
            out.stderr
        );
    }
}

/// Every command refuses a positional argument.
#[test]
fn every_command_rejects_a_positional_argument() {
    for command in COMMANDS {
        let mut args = valid_args(command);
        args.push("positional".to_owned());
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} positional argument"));
        assert!(
            out.stderr.contains("unexpected argument"),
            "{command} diagnostic: {}",
            out.stderr
        );
    }
}

/// Every command refuses a missing required flag.
#[test]
fn every_command_rejects_a_missing_flag() {
    for command in COMMANDS {
        let mut args = valid_args(command);
        // Drop the first flag and its value.
        args.remove(1);
        args.remove(1);
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} missing flag"));
        assert!(
            out.stderr.contains("missing required"),
            "{command} diagnostic: {}",
            out.stderr
        );
    }
}

/// Every command accepts the `--flag=value` form.
#[test]
fn every_command_accepts_the_equals_flag_value_form() {
    let root = temp_root("equals-form");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let out = eggpack(&to_equals_form(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ])));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "check: ok\n");
    for command in COMMANDS {
        let out = eggpack(&to_equals_form(&valid_args(command)));
        assert!(
            !out.stderr.contains("unknown option")
                && !out.stderr.contains("unexpected argument")
                && !out.stderr.contains("missing required")
                && !out.stderr.contains("only once")
                && !out.stderr.contains("must not be empty"),
            "{command} must accept the --flag=value form: {}",
            out.stderr
        );
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// Every command refuses an oversized input document.
#[test]
fn every_command_rejects_an_oversized_input() {
    let root = temp_root("oversized");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let oversized = write_file(&root.join("oversized.toml"), &"x".repeat(1_000_001));
    let oversized_plan = write_file(&root.join("oversized-plan.json"), &"x".repeat(1_000_001));
    let cases: Vec<(&str, Vec<String>)> = vec![
        (
            "check",
            argv(&[
                "check",
                "--contract",
                &oversized,
                "--pack-config",
                &inputs.pack_config,
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--selected",
                ALIAS,
                "--release-id",
                RELEASE_ID,
                "--source-revision",
                REVISION,
            ]),
        ),
        (
            "plan",
            argv(&[
                "plan",
                "--contract",
                &oversized,
                "--pack-config",
                &inputs.pack_config,
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--selected",
                ALIAS,
                "--release-id",
                RELEASE_ID,
                "--source-revision",
                REVISION,
                "--output",
                &root.join("plan.json").to_string_lossy(),
            ]),
        ),
        (
            "package",
            argv(&[
                "package",
                "--release-plan",
                &oversized_plan,
                "--build-bindings",
                &inputs.build_bindings,
                "--source-root",
                &root.to_string_lossy(),
                "--output-root",
                &root.join("out").to_string_lossy(),
            ]),
        ),
        (
            "qualify",
            argv(&[
                "qualify",
                "--contract",
                &oversized,
                "--release-plan",
                "release-plan.json",
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--artifacts-dir",
                "candidates",
                "--output-dir",
                "qualified",
            ]),
        ),
        (
            "collect",
            argv(&[
                "collect",
                "--contract",
                &oversized,
                "--release-plan",
                "release-plan.json",
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--artifacts-dir",
                "candidates",
                "--output-root",
                "finalized",
            ]),
        ),
        (
            "manifest",
            argv(&[
                "manifest",
                "--contract",
                &oversized,
                "--release-plan",
                "release-plan.json",
                "--finalized-root",
                "finalized",
                "--output",
                "release-manifest.json",
            ]),
        ),
    ];
    for (command, args) in cases {
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} oversized input"));
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// Every command refuses a symlinked output path.
#[test]
#[cfg(unix)]
fn every_command_rejects_a_symlinked_output_path() {
    use std::os::unix::fs::symlink;

    let root = temp_root("symlink");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let real = write_file(&root.join("real.json"), "{}");
    let link = root.join("link.json");
    symlink(&real, &link).unwrap();
    let cases: Vec<(&str, Vec<String>)> = vec![
        (
            "plan",
            argv(&[
                "plan",
                "--contract",
                &inputs.contract,
                "--pack-config",
                &inputs.pack_config,
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--selected",
                ALIAS,
                "--release-id",
                RELEASE_ID,
                "--source-revision",
                REVISION,
                "--output",
                &link.to_string_lossy(),
            ]),
        ),
        (
            "manifest",
            argv(&[
                "manifest",
                "--contract",
                &inputs.contract,
                "--release-plan",
                "release-plan.json",
                "--finalized-root",
                "finalized",
                "--output",
                &link.to_string_lossy(),
            ]),
        ),
    ];
    for (command, args) in cases {
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} symlinked output"));
    }
    // A symlinked output directory is refused too: the caller's root is
    // written into directly, never followed.
    let real_dir = root.join("real-dir");
    std::fs::create_dir_all(&real_dir).unwrap();
    let dir_link = root.join("link-dir");
    symlink(&real_dir, &dir_link).unwrap();
    let dir_cases: Vec<(&str, Vec<String>)> = vec![
        (
            "package",
            argv(&[
                "package",
                "--release-plan",
                "release-plan.json",
                "--build-bindings",
                &inputs.build_bindings,
                "--source-root",
                &root.to_string_lossy(),
                "--output-root",
                &dir_link.to_string_lossy(),
            ]),
        ),
        (
            "qualify",
            argv(&[
                "qualify",
                "--contract",
                &inputs.contract,
                "--release-plan",
                "release-plan.json",
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--artifacts-dir",
                "candidates",
                "--output-dir",
                &dir_link.to_string_lossy(),
            ]),
        ),
        (
            "collect",
            argv(&[
                "collect",
                "--contract",
                &inputs.contract,
                "--release-plan",
                "release-plan.json",
                "--build-bindings",
                &inputs.build_bindings,
                "--qualification-bindings",
                &inputs.qualification_bindings,
                "--artifacts-dir",
                "candidates",
                "--output-root",
                &dir_link.to_string_lossy(),
            ]),
        ),
    ];
    for (command, args) in dir_cases {
        let out = eggpack(&args);
        assert_failure(&out, &format!("{command} symlinked output directory"));
    }
    std::fs::remove_dir_all(&root).unwrap();
}

/// `--help` names every command and states its class.
#[test]
fn help_names_every_command_and_its_class() {
    let out = eggpack(&argv(&["--help"]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    for (command, class) in [
        ("check", "validation"),
        ("plan", "validation"),
        ("package", "build"),
        ("qualify", "build"),
        ("collect", "release aggregation"),
        ("manifest", "final-bytes evidence"),
    ] {
        assert!(
            out.stdout.contains(command),
            "top-level help must name {command}: {}",
            out.stdout
        );
        assert!(
            out.stdout.contains(class),
            "top-level help must state the {command} class: {}",
            out.stdout
        );
    }
    assert!(
        out.stdout
            .contains("No command in this set stages or publishes"),
        "top-level help must state that nothing stages or publishes: {}",
        out.stdout
    );
    for command in COMMANDS {
        let out = eggpack(&argv(&[command, "--help"]));
        assert_eq!(out.code, 0, "{command} --help stderr: {}", out.stderr);
        assert!(
            out.stdout.contains(command),
            "{command} --help must name the command: {}",
            out.stdout
        );
        assert!(
            out.stdout
                .contains("No command in this set stages or publishes"),
            "{command} --help must state that nothing stages or publishes: {}",
            out.stdout
        );
    }
}

/// The historical command families keep their documented behaviour.
#[test]
fn existing_command_families_still_behave() {
    let root = temp_root("existing");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);

    // `eggpack contract expand` still answers one scalar on stdout.
    let out = eggpack(&argv(&[
        "contract",
        "expand",
        "--contract",
        &inputs.contract,
        "--release-id",
        RELEASE_ID,
        "--target",
        ALIAS,
        "--field",
        "asset",
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, format!("eggsact-{RELEASE_ID}-{TRIPLE}\n"));
    assert_eq!(out.stderr, "");

    // `eggpack ci _resolve-release` still writes the three runtime documents.
    let ci_plan = root.join("ci-plan.json");
    let policy = root.join("github.json");
    let out = eggpack(&argv(&[
        "ci",
        "_resolve-release",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--tag",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--template",
        &inputs.template,
        "--source-root",
        &repo.to_string_lossy(),
        "--output-plan",
        &root.join("plan.json").to_string_lossy(),
        "--output-ci-plan",
        &ci_plan.to_string_lossy(),
        "--output-github-policy",
        &policy.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert!(ci_plan.exists());
    assert!(policy.exists());

    // `eggpack ci generate` still writes a workflow, and `ci check` still
    // accepts the unmodified workflow.
    let provider_policy = write_file(&root.join("provider-policy.json"), GITHUB_POLICY);
    let workflow = root.join("workflow.yml");
    let out = eggpack(&argv(&[
        "ci",
        "generate",
        "--ci-plan",
        &ci_plan.to_string_lossy(),
        "--github-policy",
        &provider_policy,
        "--output",
        &workflow.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "generate stderr: {}", out.stderr);
    assert!(workflow.exists());
    assert!(
        out.stdout.starts_with("generated "),
        "generate stdout: {}",
        out.stdout
    );
    let out = eggpack(&argv(&[
        "ci",
        "check",
        "--ci-plan",
        &ci_plan.to_string_lossy(),
        "--github-policy",
        &provider_policy,
        "--workflow",
        &workflow.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "check stderr: {}", out.stderr);
    assert!(
        out.stdout.starts_with("ci check: match"),
        "check stdout: {}",
        out.stdout
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// The SHA-256 digest of one file's bytes.
fn file_digest(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("read file");
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, &bytes);
    sha2::Digest::finalize(hasher)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Resolve a release plan and build its candidates under the caller's root.
///
/// Returns the plan document path and the candidate root the build wrote.
fn plan_and_package(
    root: &Path,
    inputs: &Inputs,
    source: &Path,
    revision: &str,
    alias: &str,
) -> (PathBuf, PathBuf) {
    let plan_path = root.join("release-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        alias,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        revision,
        "--output",
        &plan_path.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "plan stderr: {}", out.stderr);
    let candidates = root.join("candidates");
    let out = eggpack(&argv(&[
        "package",
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--source-root",
        &source.to_string_lossy(),
        "--output-root",
        &candidates.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "package stderr: {}", out.stderr);
    (plan_path, candidates)
}

/// `eggpack qualify` refuses to stage its artifacts into the directory it
/// reads its candidates from, and leaves the candidate bytes untouched.
///
/// The candidate bytes are the input of the qualification: staging them into
/// the same directory would rewrite the very files the qualification is about.
#[test]
fn qualify_refuses_to_stage_into_its_own_artifacts_directory() {
    let root = temp_root("qualify-same-dir");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);
    let (plan_path, candidates) = plan_and_package(&root, &inputs, &source, &revision, &alias);

    let candidate = candidates
        .join(&triple)
        .join("candidates")
        .join("candidate-direct");
    assert!(candidate.exists(), "the build must produce a candidate");
    let before = file_digest(&candidate);
    let before_len = std::fs::metadata(&candidate).unwrap().len();

    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &candidates.to_string_lossy(),
    ]));
    assert_failure(&out, "qualify into its own artifacts directory");
    assert!(
        out.stderr
            .contains("output dir must differ from artifacts dir"),
        "the diagnostic must name the collision: {}",
        out.stderr
    );

    // The input bytes are unchanged: not truncated, not replaced.
    assert_eq!(
        std::fs::metadata(&candidate).unwrap().len(),
        before_len,
        "the candidate file must keep its length"
    );
    assert_eq!(
        file_digest(&candidate),
        before,
        "the candidate bytes must be unchanged"
    );
    assert!(
        !candidates.join(&triple).join("evidence.json").exists(),
        "the refused qualification must not write evidence into the input tree"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// The `ci _qualify-target` runner refuses the same collision: staging the
/// candidates into the directory they were read from would truncate them.
///
/// The runner reads its candidates from the directory the handoff names and
/// stages them under `candidates/` inside its output directory. When both
/// name one directory, the staged file and the source file are one file.
#[test]
fn ci_qualify_target_refuses_to_stage_into_its_own_candidate_directory() {
    let root = temp_root("ci-qualify-same-dir");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);
    let (plan_path, candidates) = plan_and_package(&root, &inputs, &source, &revision, &alias);

    let candidate_dir = candidates.join(&triple).join("candidates");
    let candidate = candidate_dir.join("candidate-direct");
    assert!(candidate.exists(), "the build must produce a candidate");
    // The runner stages candidates under `candidates/` inside its output
    // directory, so the same directory holds the source bytes and the
    // staging destination at once.
    let staged = candidate_dir.join("candidates");
    std::fs::create_dir_all(&staged).unwrap();
    std::fs::copy(&candidate, staged.join("candidate-direct")).unwrap();
    let before = file_digest(&candidate);
    let before_len = std::fs::metadata(&candidate).unwrap().len();

    let out = eggpack(&argv(&[
        "ci",
        "_qualify-target",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--target",
        &triple,
        "--candidate-dir",
        &candidate_dir.to_string_lossy(),
        "--build-handoff",
        &candidate_dir
            .parent()
            .unwrap()
            .join("build-handoff.json")
            .to_string_lossy(),
        "--output-dir",
        &candidate_dir.to_string_lossy(),
    ]));
    assert_eq!(out.code, 1, "the runner must fail closed");
    assert!(
        out.stderr
            .contains("candidate staging source and destination are the same file"),
        "the diagnostic must name the collision: {}",
        out.stderr
    );
    assert_eq!(
        std::fs::metadata(&candidate).unwrap().len(),
        before_len,
        "the candidate file must keep its length"
    );
    assert_eq!(
        file_digest(&candidate),
        before,
        "the candidate bytes must be unchanged"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// One consumer validator map for the direct target of one release.
fn consumer_validators_json(triple: &str) -> String {
    format!(
        "{{\"{triple}\": {{\"schema_version\": 1, \"selector\": {{\"kind\": \"direct\"}}, \"interpreter\": \"python3\", \"script\": \"tools/validate.py\", \"timeout_ms\": 1000, \"stdout_limit\": 1024, \"stderr_limit\": 1024}}}}"
    )
}

/// `eggpack qualify` accepts a QEMU sysroot and reaches the same verdict as
/// the `ci _qualify-target` runner.
///
/// The sysroot is what an emulated qualification runs against, so a target
/// qualified through the operator must be able to carry the same sysroot the
/// CI path carries — and must refuse it under the same rule.
#[test]
fn qualify_accepts_a_qemu_sysroot_and_matches_the_ci_runner() {
    let root = temp_root("qualify-sysroot");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);
    let (plan_path, candidates) = plan_and_package(&root, &inputs, &source, &revision, &alias);

    let sysroot = root.join("sysroot");
    std::fs::create_dir_all(&sysroot).unwrap();
    let qualified = root.join("qualified");
    let operator = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &qualified.to_string_lossy(),
        "--qemu-sysroot",
        &sysroot.to_string_lossy(),
    ]));
    let runner = eggpack(&argv(&[
        "ci",
        "_qualify-target",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--target",
        &triple,
        "--candidate-dir",
        &candidates
            .join(&triple)
            .join("candidates")
            .to_string_lossy(),
        "--build-handoff",
        &candidates
            .join(&triple)
            .join("build-handoff.json")
            .to_string_lossy(),
        "--output-dir",
        &root.join("runner-qualified").to_string_lossy(),
        "--qemu-sysroot",
        &sysroot.to_string_lossy(),
    ]));
    // A native qualification refuses the sysroot on both surfaces, for the
    // same reason: the flag is carried, not dropped.
    assert_eq!(operator.code, 1, "operator stderr: {}", operator.stderr);
    assert_eq!(runner.code, 1, "runner stderr: {}", runner.stderr);
    assert!(
        operator
            .stderr
            .contains("QEMU sysroot is only valid for Emulated qualification"),
        "the operator diagnostic must name the rule: {}",
        operator.stderr
    );
    assert!(
        runner.stderr.starts_with("eggpack: "),
        "the runner must refuse the same sysroot: {}",
        runner.stderr
    );

    // A sysroot that is not a real directory is refused before the
    // qualification runs.
    let file = write_file(&root.join("not-a-dir"), "x");
    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &root.join("other").to_string_lossy(),
        "--qemu-sysroot",
        &file,
    ]));
    assert_failure(&out, "a sysroot that is not a directory");
    assert!(
        out.stderr.contains("qemu sysroot is not a real directory"),
        "the diagnostic must name the refusal: {}",
        out.stderr
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack plan` writes the same bytes as the `ci _resolve-release` runner in
/// the v-prefixed stable semver identity mode.
#[test]
fn plan_writes_the_same_bytes_as_ci_resolve_release_in_v_prefixed_mode() {
    let root = temp_root("plan-v-prefixed");
    let revision = init_git_repo(&root, RELEASE_ID);
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let runner_plan = root.join("runner-plan.json");
    let out = eggpack(&argv(&[
        "ci",
        "_resolve-release",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--tag",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--template",
        &inputs.template,
        "--source-root",
        &root.to_string_lossy(),
        "--identity-mode",
        "v_prefixed_stable_semver",
        "--output-plan",
        &runner_plan.to_string_lossy(),
        "--output-ci-plan",
        &root.join("runner-ci-plan.json").to_string_lossy(),
        "--output-github-policy",
        &root.join("runner-github.json").to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);

    let local_plan = root.join("local-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--identity-mode",
        "v_prefixed_stable_semver",
        "--output",
        &local_plan.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    assert_eq!(out.stdout, "plan: ok\n");
    assert_eq!(
        std::fs::read(&local_plan).unwrap(),
        std::fs::read(&runner_plan).unwrap(),
        "both surfaces must write identical plan bytes in the v-prefixed mode"
    );
    // The mode maps `vMAJOR.MINOR.PATCH` to `MAJOR.MINOR.PATCH`.
    let plan: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&local_plan).unwrap()).unwrap();
    assert_eq!(plan["release_id"], "1.2.3");
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` refuses an unknown release identity mode.
#[test]
fn check_rejects_an_unknown_identity_mode() {
    let root = temp_root("check-identity-mode");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ALIAS,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
        "--identity-mode",
        "semver",
    ]));
    assert_failure(&out, "an unknown identity mode");
    assert!(
        out.stderr.contains("unknown release identity mode"),
        "the diagnostic must name the refusal: {}",
        out.stderr
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// A consumer-validated release is planned and aggregated locally, exactly as
/// the CI path aggregates it.
#[test]
fn consumer_validated_release_plans_and_aggregates_locally() {
    let root = temp_root("consumer-validated");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);
    let (plan_path, candidates) = plan_and_package(&root, &inputs, &source, &revision, &alias);

    let validators = write_file(
        &root.join("consumer-validators.json"),
        &consumer_validators_json(&triple),
    );

    // The planning surface accepts the validator map and validates it against
    // the release it describes.
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        &alias,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--consumer-validators",
        &validators,
    ]));
    assert_eq!(out.code, 0, "check stderr: {}", out.stderr);
    assert_eq!(out.stdout, "check: ok\n");

    // Qualify the candidates.
    let qualified = root.join("qualified");
    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &qualified.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "qualify stderr: {}", out.stderr);

    // Without the consumer evidence the aggregation fails closed.
    let local = root.join("local");
    std::fs::create_dir_all(&local).unwrap();
    let out = eggpack(&argv(&[
        "collect",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &local.join("finalized").to_string_lossy(),
        "--consumer-validators",
        &validators,
    ]));
    assert_failure(
        &out,
        "a consumer-validated release without consumer evidence",
    );
    assert!(
        out.stderr.contains("consumer evidence"),
        "the diagnostic must name the missing evidence: {}",
        out.stderr
    );

    // With the consumer evidence the release aggregates.
    let candidate = qualified
        .join(&triple)
        .join("candidates")
        .join("candidate-direct");
    let evidence = format!(
        "{{\"schema_version\": 1, \"release_id\": \"{RELEASE_ID}\", \"source_revision\": \"{revision}\", \"target\": \"{triple}\", \"selector\": {{\"kind\": \"direct\"}}, \"interpreter\": \"python3\", \"outcome\": \"passed\", \"candidate_size\": {}, \"candidate_sha256\": \"{}\"}}",
        std::fs::metadata(&candidate).unwrap().len(),
        file_digest(&candidate),
    );
    write_file(
        &qualified.join(&triple).join("consumer-evidence.json"),
        &evidence,
    );
    let out = eggpack(&argv(&[
        "collect",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &local.join("finalized").to_string_lossy(),
        "--consumer-validators",
        &validators,
    ]));
    assert_eq!(out.code, 0, "collect stderr: {}", out.stderr);
    assert_eq!(out.stdout, "collect: ok\n");
    assert!(local.join("release-manifest.json").exists());
    std::fs::remove_dir_all(&root).unwrap();
}

/// The operator reads its input documents as JSON exactly as the CI path does.
#[test]
fn operator_reads_json_input_documents() {
    let root = temp_root("json-inputs");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let pack_config = write_file(
        &root.join("pack.json"),
        &format!(
            "{{\"schema_version\": 1, \"targets\": [{{\"target\": \"{triple}\", \"strategy\": \"native_cargo\", \"host_os\": \"{host_os}\", \"host_arch\": \"{host_arch}\", \"toolchain\": {{\"rust\": \"stable\"}}, \"floor\": {{\"kind\": \"none\"}}, \"qualification\": \"structural\", \"support\": \"required\"}}]}}"
        ),
    );
    let build_bindings = write_file(
        &root.join("build.json"),
        &format!(
            "{{\"schema_version\": 1, \"targets\": {{\"{triple}\": [{{\"selector\": {{\"kind\": \"direct\"}}, \"package\": \"eggsact\", \"binary\": \"bin0\"}}]}}}}"
        ),
    );
    let qualification_bindings = write_file(
        &root.join("qualification.json"),
        &format!("{{\"schema_version\": 1, \"targets\": {{\"{triple}\": {{}}}}}}"),
    );

    // The JSON documents resolve to the same plan as the TOML documents.
    let json_plan = root.join("json-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &pack_config,
        "--build-bindings",
        &build_bindings,
        "--qualification-bindings",
        &qualification_bindings,
        "--selected",
        &alias,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
        "--output",
        &json_plan.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "plan stderr: {}", out.stderr);
    assert_eq!(out.stdout, "plan: ok\n");

    let toml_plan = root.join("toml-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        &alias,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
        "--output",
        &toml_plan.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "plan stderr: {}", out.stderr);
    assert_eq!(
        std::fs::read(&json_plan).unwrap(),
        std::fs::read(&toml_plan).unwrap(),
        "the JSON and TOML documents must resolve to the same plan"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// Write an archive producer input set for the host target.
///
/// The archive carries two members, so the derived manifest must describe the
/// member bytes the finalized archive actually contains.
fn write_archive_inputs(root: &Path, triple: &str, alias: &str, host_os: &str, host_arch: &str) {
    write_file(
        &root.join("contract.toml"),
        &format!(
            "schema_version = 1\n\n[product]\nid = \"egress\"\ndisplay_name = \"Egress\"\n\n[[targets]]\ntriple = \"{triple}\"\naliases = [\"{alias}\"]\n\n[targets.asset]\nkind = \"archive\"\nasset = \"{{product}}-{{version}}-{{target}}.tar.gz\"\n\n[[targets.asset.members]]\nsource = \"egress\"\ninstall = \"egress\"\n\n[[targets.asset.members]]\nsource = \"bin/egress-helper\"\ninstall = \"egress-helper\"\n\n[targets.checksum]\nsidecar = \"{{asset}}.sha256\"\n"
        ),
    );
    write_file(
        &root.join("pack.toml"),
        &format!(
            "schema_version = 1\n\n[[targets]]\ntarget = \"{triple}\"\nstrategy = \"native_cargo\"\nhost_os = \"{host_os}\"\nhost_arch = \"{host_arch}\"\nqualification = \"structural\"\nsupport = \"required\"\n\n[targets.toolchain]\nrust = \"stable\"\n\n[targets.floor]\nkind = \"none\"\n"
        ),
    );
    write_file(
        &root.join("build.toml"),
        &format!(
            "schema_version = 1\n\n[[targets.\"{triple}\"]]\nselector = {{ kind = \"archive_member\", source = \"egress\" }}\npackage = \"eggsact\"\nbinary = \"bin0\"\n\n[[targets.\"{triple}\"]]\nselector = {{ kind = \"archive_member\", source = \"bin/egress-helper\" }}\npackage = \"eggsact\"\nbinary = \"bin1\"\n"
        ),
    );
    write_file(
        &root.join("qualification.toml"),
        &format!("schema_version = 1\n\n[targets.\"{triple}\"]\n"),
    );
}

/// Write a Cargo project that builds the two binaries an archive carries.
fn write_cargo_fixture_with_two_binaries(root: &Path) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"eggsact\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"bin0\"\npath = \"src/bin0.rs\"\n\n[[bin]]\nname = \"bin1\"\npath = \"src/bin1.rs\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Cargo.lock"),
        "version = 3\n\n[[package]]\nname = \"eggsact\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/bin0.rs"), "fn main() {}\n").unwrap();
    std::fs::write(root.join("src/bin1.rs"), "fn main() {}\n").unwrap();
}

/// `eggpack manifest` refuses a tampered artifacts directory instead of
/// reporting a manifest that describes bytes the release does not contain.
#[test]
fn manifest_refuses_a_tampered_artifacts_directory() {
    let root = temp_root("manifest-tampered");
    let (triple, alias, host_os, host_arch) = host_target();
    write_archive_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture_with_two_binaries(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);

    let plan_path = root.join("release-plan.json");
    let out = eggpack(&argv(&[
        "plan",
        "--contract",
        &root.join("contract.toml").to_string_lossy(),
        "--pack-config",
        &root.join("pack.toml").to_string_lossy(),
        "--build-bindings",
        &root.join("build.toml").to_string_lossy(),
        "--qualification-bindings",
        &root.join("qualification.toml").to_string_lossy(),
        "--selected",
        &alias,
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        &revision,
        "--output",
        &plan_path.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "plan stderr: {}", out.stderr);

    let candidates = root.join("candidates");
    let out = eggpack(&argv(&[
        "package",
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &root.join("build.toml").to_string_lossy(),
        "--source-root",
        &source.to_string_lossy(),
        "--output-root",
        &candidates.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "package stderr: {}", out.stderr);

    let qualified = root.join("qualified");
    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &root.join("contract.toml").to_string_lossy(),
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &root.join("build.toml").to_string_lossy(),
        "--qualification-bindings",
        &root.join("qualification.toml").to_string_lossy(),
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &qualified.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "qualify stderr: {}", out.stderr);

    let local = root.join("local");
    std::fs::create_dir_all(&local).unwrap();
    let finalized = local.join("finalized");
    let out = eggpack(&argv(&[
        "collect",
        "--contract",
        &root.join("contract.toml").to_string_lossy(),
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &root.join("build.toml").to_string_lossy(),
        "--qualification-bindings",
        &root.join("qualification.toml").to_string_lossy(),
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &finalized.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "collect stderr: {}", out.stderr);
    let collected = local.join("release-manifest.json");
    assert!(collected.exists());

    // The untampered tree derives exactly the collected manifest.
    let derived = root.join("derived-manifest.json");
    let out = eggpack(&argv(&[
        "manifest",
        "--contract",
        &root.join("contract.toml").to_string_lossy(),
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &root.join("build.toml").to_string_lossy(),
        "--qualification-bindings",
        &root.join("qualification.toml").to_string_lossy(),
        "--finalized-root",
        &finalized.to_string_lossy(),
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output",
        &derived.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "manifest stderr: {}", out.stderr);
    assert_eq!(out.stdout, "manifest: ok\n");
    assert_eq!(
        std::fs::read(&derived).unwrap(),
        std::fs::read(&collected).unwrap(),
        "the derived manifest must equal the collected manifest"
    );

    // A tampered member in the artifacts directory is refused: the digest the
    // manifest would record no longer describes the finalized archive.
    let member_dir = qualified.join(&triple).join("candidates");
    let tampered = std::fs::read_dir(&member_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().starts_with("candidate-archive-"))
                .unwrap_or(false)
        })
        .expect("the build must produce an archive member candidate");
    std::fs::write(&tampered, b"tampered member bytes").unwrap();

    let out = eggpack(&argv(&[
        "manifest",
        "--contract",
        &root.join("contract.toml").to_string_lossy(),
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &root.join("build.toml").to_string_lossy(),
        "--qualification-bindings",
        &root.join("qualification.toml").to_string_lossy(),
        "--finalized-root",
        &finalized.to_string_lossy(),
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output",
        &root.join("tampered-manifest.json").to_string_lossy(),
    ]));
    assert_failure(&out, "a tampered artifacts directory");
    assert!(
        out.stderr.contains("does not match the finalized archive"),
        "the diagnostic must name the mismatch: {}",
        out.stderr
    );
    assert!(
        !root.join("tampered-manifest.json").exists(),
        "a refused manifest must not be written"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack manifest` refuses a finalized tree whose collected manifest
/// differs from the derived one.
#[test]
fn manifest_refuses_a_collected_manifest_that_differs() {
    let root = temp_root("manifest-collected-differs");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);
    let (plan_path, candidates) = plan_and_package(&root, &inputs, &source, &revision, &alias);

    let qualified = root.join("qualified");
    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &qualified.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "qualify stderr: {}", out.stderr);

    let local = root.join("local");
    std::fs::create_dir_all(&local).unwrap();
    let finalized = local.join("finalized");
    let out = eggpack(&argv(&[
        "collect",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &finalized.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "collect stderr: {}", out.stderr);
    let collected = local.join("release-manifest.json");

    // The derived manifest and the collected manifest are the same document,
    // evidence references included.
    let derived = root.join("derived-manifest.json");
    let out = eggpack(&argv(&[
        "manifest",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--finalized-root",
        &finalized.to_string_lossy(),
        "--output",
        &derived.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "manifest stderr: {}", out.stderr);
    let collected_document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&collected).unwrap()).unwrap();
    let derived_document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&derived).unwrap()).unwrap();
    assert_eq!(
        derived_document["evidence_references"], collected_document["evidence_references"],
        "the derived and collected manifests must agree on the evidence references"
    );

    // A collected manifest that describes another revision is a different
    // release, and the derived manifest must not be written beside it.
    let collected = local.join("release-manifest.json");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&collected).unwrap()).unwrap();
    document["source_revision"] =
        serde_json::Value::String("0123456789abcdef0123456789abcdef01234567".to_owned());
    std::fs::write(&collected, serde_json::to_string(&document).unwrap()).unwrap();

    let out = eggpack(&argv(&[
        "manifest",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--finalized-root",
        &finalized.to_string_lossy(),
        "--output",
        &root.join("refused-manifest.json").to_string_lossy(),
    ]));
    assert_failure(&out, "a collected manifest that differs");
    assert!(
        out.stderr
            .contains("derived manifest differs from the collected release manifest"),
        "the diagnostic must name the difference: {}",
        out.stderr
    );
    assert!(
        !root.join("refused-manifest.json").exists(),
        "a refused manifest must not be written"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack collect` names the reason its aggregation failed.
#[test]
fn collect_names_the_aggregation_failure_reason() {
    let root = temp_root("collect-reason");
    let (triple, alias, host_os, host_arch) = host_target();
    let inputs = write_inputs(&root, &triple, &alias, host_os, host_arch);
    let source = root.join("source");
    write_cargo_fixture(&source);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let revision = init_git_repo(&repo, RELEASE_ID);
    let (plan_path, candidates) = plan_and_package(&root, &inputs, &source, &revision, &alias);

    let qualified = root.join("qualified");
    let out = eggpack(&argv(&[
        "qualify",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &candidates.to_string_lossy(),
        "--output-dir",
        &qualified.to_string_lossy(),
    ]));
    assert_eq!(out.code, 0, "qualify stderr: {}", out.stderr);

    // The candidate bytes no longer match the qualification evidence, so the
    // aggregation is refused — with the reason, not a bare failure.
    let candidate = qualified
        .join(&triple)
        .join("candidates")
        .join("candidate-direct");
    let bytes = std::fs::read(&candidate).unwrap();
    let mut tampered = bytes.clone();
    tampered[0] ^= 0xff;
    std::fs::write(&candidate, &tampered).unwrap();

    let local = root.join("local");
    std::fs::create_dir_all(&local).unwrap();
    let out = eggpack(&argv(&[
        "collect",
        "--contract",
        &inputs.contract,
        "--release-plan",
        &plan_path.to_string_lossy(),
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--artifacts-dir",
        &qualified.to_string_lossy(),
        "--output-root",
        &local.join("finalized").to_string_lossy(),
    ]));
    assert_failure(&out, "an aggregation that cannot finalize");
    assert!(
        out.stderr.starts_with("eggpack: aggregation failed: "),
        "the diagnostic must carry the cause: {}",
        out.stderr
    );
    assert!(
        out.stderr.len() > "eggpack: aggregation failed: ".len(),
        "the diagnostic must name a cause: {}",
        out.stderr
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// `eggpack check` refuses an empty selected target alias.
#[test]
fn check_rejects_an_empty_selected_alias() {
    let root = temp_root("check-empty-alias");
    let inputs = write_inputs(&root, TRIPLE, ALIAS, "linux", "x86_64");
    let out = eggpack(&argv(&[
        "check",
        "--contract",
        &inputs.contract,
        "--pack-config",
        &inputs.pack_config,
        "--build-bindings",
        &inputs.build_bindings,
        "--qualification-bindings",
        &inputs.qualification_bindings,
        "--selected",
        ",",
        "--release-id",
        RELEASE_ID,
        "--source-revision",
        REVISION,
    ]));
    assert_failure(&out, "an empty selected alias");
    assert!(
        out.stderr
            .contains("selected target aliases must not be empty"),
        "the diagnostic must name the empty alias: {}",
        out.stderr
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// The top-level dispatch prints the usage block, not one diagnostic line.
#[test]
fn the_top_level_dispatch_prints_the_usage_block() {
    let out = eggpack(&argv(&[]));
    assert_eq!(out.code, 1, "an empty invocation must fail");
    assert!(
        out.stderr.lines().count() > 1,
        "the usage block spans several lines: {}",
        out.stderr
    );
    let unknown = eggpack(&argv(&["bogus"]));
    assert_eq!(unknown.code, 1, "an unknown command must fail");
    assert!(
        unknown.stderr.lines().count() > 1,
        "the usage block spans several lines: {}",
        unknown.stderr
    );
}
