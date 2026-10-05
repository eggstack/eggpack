//! Process-boundary contract for `eggpack contract expand` (Contract M003).
//!
//! The unit tests inside the binary can only observe `contract_expand`'s
//! `Result`. The documented stdout/stderr contract — exactly one scalar plus
//! a trailing newline on stdout, diagnostics only on stderr, nonzero exit on
//! every failure — is only observable from outside the process, so it is
//! asserted here.
//!
//! These tests are hermetic: they only read fixtures checked into this
//! repository and a temporary directory. No network, no repository
//! discovery, no release selection.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const SIMPLE: &str = include_str!("../../eggpack-contract/tests/fixtures/simple-direct.toml");
const CONSUMER: &str =
    include_str!("../../eggpack-contract/tests/fixtures/consumer-direct-targets.toml");
const BUNDLE: &str = include_str!("../../eggpack-contract/tests/fixtures/codegg-bundle.toml");
const ARCHIVE: &str = include_str!("../../eggpack-contract/tests/fixtures/egress-archive.toml");

static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> PathBuf {
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "eggpack-cli-contract-expand-{label}-{}-{id}",
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

fn write_contract(root: &Path, name: &str, text: &str) -> String {
    let path = root.join(name);
    std::fs::write(&path, text).unwrap();
    path.to_string_lossy().into_owned()
}

fn argv(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn expand(contract: &str, release: &str, target: &str, field: &str) -> Vec<String> {
    [
        "contract".to_owned(),
        "expand".to_owned(),
        "--contract".to_owned(),
        contract.to_owned(),
        "--release-id".to_owned(),
        release.to_owned(),
        "--target".to_owned(),
        target.to_owned(),
        "--field".to_owned(),
        field.to_owned(),
    ]
    .to_vec()
}

/// Successful query: exit 0, stdout is exactly the scalar plus one newline,
/// stderr empty.
fn assert_scalar(contract: &str, target: &str, field: &str, expected: &str) {
    let out = eggpack(&expand(contract, "v1.2.7", target, field));
    assert_eq!(out.code, 0, "{target}/{field} stderr: {}", out.stderr);
    assert_eq!(out.stdout, format!("{expected}\n"), "{target}/{field}");
    assert_eq!(out.stderr, "", "{target}/{field} must not write to stderr");
}

/// Failed query: nonzero exit, stdout empty, stderr non-empty and bounded.
fn assert_fails(contract: &str, target: &str, field: &str) {
    let out = eggpack(&expand(contract, "v1.2.7", target, field));
    assert_ne!(out.code, 0, "{target}/{field} must exit nonzero");
    assert_eq!(
        out.stdout, "",
        "{target}/{field} must print no partial scalar"
    );
    assert!(
        !out.stderr.is_empty(),
        "{target}/{field} must explain itself"
    );
    assert!(
        out.stderr.len() <= 1024,
        "{target}/{field} diagnostic must stay bounded, got {} bytes",
        out.stderr.len()
    );
}

#[test]
fn success_prints_exactly_one_scalar_and_nothing_else() {
    let root = temp_root("success");
    let simple = write_contract(&root, "simple.toml", SIMPLE);
    let consumer = write_contract(&root, "consumer.toml", CONSUMER);

    assert_scalar(
        &simple,
        "linux-x64",
        "canonical-target",
        "x86_64-unknown-linux-gnu",
    );
    assert_scalar(
        &simple,
        "linux-x64",
        "asset",
        "eggsact-v1.2.7-x86_64-unknown-linux-gnu",
    );
    assert_scalar(
        &simple,
        "linux-x64",
        "sidecar",
        "eggsact-v1.2.7-x86_64-unknown-linux-gnu.sha256",
    );
    assert_scalar(&simple, "linux-x64", "install", "eggsact");

    // The live consumer contract shape: version-free public asset names.
    assert_scalar(
        &consumer,
        "windows-x64",
        "asset",
        "eggsact-x86_64-pc-windows-msvc.exe",
    );
    assert_scalar(
        &consumer,
        "windows-x64",
        "sidecar",
        "eggsact-x86_64-pc-windows-msvc.exe.sha256",
    );
    assert_scalar(&consumer, "windows-x64", "install", "eggsact.exe");
    assert_scalar(
        &consumer,
        "linux-x64",
        "canonical-target",
        "x86_64-unknown-linux-gnu",
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn repeated_invocations_are_byte_identical() {
    let root = temp_root("deterministic");
    let consumer = write_contract(&root, "consumer.toml", CONSUMER);
    let args = expand(&consumer, "v1.2.7", "linux-arm64", "asset");
    let first = eggpack(&args).stdout;
    for _ in 0..4 {
        assert_eq!(eggpack(&args).stdout, first);
    }
    assert_eq!(first, "eggsact-aarch64-unknown-linux-gnu\n");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_command_never_writes_to_the_contract_or_its_directory() {
    let root = temp_root("no-mutation");
    let consumer = write_contract(&root, "consumer.toml", CONSUMER);
    let before = std::fs::read(root.join("consumer.toml")).unwrap();
    let entries_before = std::fs::read_dir(&root).unwrap().count();

    for (target, field) in [
        ("linux-x64", "asset"),
        ("macos-arm64", "install"),
        ("absent", "asset"),
        ("linux-x64", "entries[0].asset"),
    ] {
        let _ = eggpack(&expand(&consumer, "v1.2.7", target, field));
    }

    assert_eq!(std::fs::read(root.join("consumer.toml")).unwrap(), before);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), entries_before);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn every_failure_mode_exits_nonzero_with_an_empty_stdout() {
    let root = temp_root("failures");
    let simple = write_contract(&root, "simple.toml", SIMPLE);
    let bundle = write_contract(&root, "bundle.toml", BUNDLE);
    let archive = write_contract(&root, "archive.toml", ARCHIVE);
    let future = write_contract(
        &root,
        "future.toml",
        &SIMPLE.replace("schema_version = 1", "schema_version = 2"),
    );
    let broken = write_contract(&root, "broken.toml", "schema_version = 1\n[product\n");
    let absent = root.join("absent.toml").to_string_lossy().into_owned();

    // Unknown target and alias.
    assert_fails(&simple, "x86_64-pc-windows-msvc", "asset");
    assert_fails(&simple, "armv7-unknown-linux-gnueabihf", "asset");
    // Unsupported schema version and malformed input.
    assert_fails(&future, "linux-x64", "asset");
    assert_fails(&broken, "linux-x64", "asset");
    // Missing and non-file inputs.
    assert_fails(&absent, "linux-x64", "asset");
    assert_fails(&root.to_string_lossy(), "linux-x64", "asset");
    // Direct-only fields on bundle and archive targets.
    for contract in [&bundle, &archive] {
        assert_fails(contract, "linux-x64", "asset");
        assert_fails(contract, "linux-x64", "sidecar");
        assert_fails(contract, "linux-x64", "install");
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn canonical_target_is_valid_for_every_asset_form() {
    let root = temp_root("canonical");
    let bundle = write_contract(&root, "bundle.toml", BUNDLE);
    let archive = write_contract(&root, "archive.toml", ARCHIVE);
    assert_scalar(
        &bundle,
        "linux-x64",
        "canonical-target",
        "x86_64-unknown-linux-gnu",
    );
    assert_scalar(
        &archive,
        "linux-x64",
        "canonical-target",
        "x86_64-unknown-linux-gnu",
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn argument_errors_exit_nonzero_without_a_scalar() {
    let root = temp_root("arguments");
    let simple = write_contract(&root, "simple.toml", SIMPLE);

    let base = expand(&simple, "v1.2.7", "linux-x64", "asset");

    // The valid baseline must succeed, so each case below adds exactly one
    // defect to a working invocation.
    assert_eq!(eggpack(&base).code, 0, "{}", eggpack(&base).stderr);

    for extra in [
        argv(&["--field", "install"]), // duplicate flag
        argv(&["--unknown", "x"]),     // unknown option
        argv(&["positional"]),         // positional argument
        argv(&["--target"]),           // missing value for the last flag
        argv(&["--field="]),           // empty value
    ] {
        let label = format!("{extra:?}");
        let mut args = base.clone();
        args.extend(extra.iter().cloned());
        let out = eggpack(&args);
        assert_ne!(out.code, 0, "{label} must exit nonzero");
        assert_eq!(out.stdout, "", "{label} must print no scalar");
        assert!(!out.stderr.is_empty(), "{label} must explain itself");
    }

    // Dropping a required flag fails closed.
    for truncated in [
        argv(&["contract", "expand"]),
        argv(&["contract", "expand", "--contract", &simple]),
    ] {
        let out = eggpack(&truncated);
        assert_ne!(out.code, 0, "{truncated:?} must exit nonzero");
        assert_eq!(out.stdout, "", "{truncated:?} must print no scalar");
        assert!(out.stderr.contains("missing required"), "{}", out.stderr);
    }

    // `--flag=value` form is accepted.
    let inline = eggpack(&argv(&[
        "contract",
        "expand",
        &format!("--contract={simple}"),
        "--release-id=v1.2.7",
        "--target=linux-x64",
        "--field=asset",
    ]));
    assert_eq!(inline.code, 0, "{}", inline.stderr);
    assert_eq!(inline.stdout, "eggsact-v1.2.7-x86_64-unknown-linux-gnu\n");

    // Top-level surface: help succeeds, unknown families fail.
    assert_eq!(eggpack(&argv(&["--help"])).code, 0);
    assert_eq!(eggpack(&argv(&["contract", "--help"])).code, 0);
    assert_eq!(eggpack(&argv(&["contract"])).code, 1);
    assert_eq!(eggpack(&argv(&["contract", "generate"])).code, 1);
    assert_eq!(eggpack(&argv(&["nope"])).code, 1);
    assert_eq!(eggpack(&[]).code, 1);

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinked_and_oversized_contracts_are_refused() {
    let root = temp_root("bounds");
    let simple = write_contract(&root, "simple.toml", SIMPLE);
    let link = root.join("link.toml");
    std::os::unix::fs::symlink(&simple, &link).unwrap();

    let out = eggpack(&expand(
        &link.to_string_lossy(),
        "v1.2.7",
        "linux-x64",
        "asset",
    ));
    assert_ne!(out.code, 0);
    assert_eq!(out.stdout, "");
    assert!(out.stderr.contains("symlink"), "{}", out.stderr);

    let oversized = root.join("oversized.toml");
    let mut padded = SIMPLE.to_owned();
    padded.push_str("# ");
    padded.push_str(&"x".repeat(1_000_000));
    std::fs::write(&oversized, &padded).unwrap();
    let out = eggpack(&expand(
        &oversized.to_string_lossy(),
        "v1.2.7",
        "linux-x64",
        "asset",
    ));
    assert_ne!(out.code, 0);
    assert_eq!(out.stdout, "");
    assert!(out.stderr.contains("size bound"), "{}", out.stderr);

    std::fs::remove_dir_all(root).unwrap();
}

/// The CLI README documents a runnable example. This test is the guard
/// against the prior unguarded-example drift finding: if the documented
/// command stops working, this fails.
#[test]
fn the_documented_readme_example_runs_verbatim() {
    let root = temp_root("readme");
    write_contract(&root, "distribution.toml", SIMPLE);
    // README form, run with the documented relative paths and the
    // documentation's own working directory.
    let output = Command::new(env!("CARGO_BIN_EXE_eggpack"))
        .current_dir(&root)
        .args([
            "contract",
            "expand",
            "--contract",
            "distribution.toml",
            "--release-id",
            "v1.2.7",
            "--target",
            "linux-x64",
            "--field",
            "asset",
        ])
        .output()
        .expect("run eggpack binary");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "eggsact-v1.2.7-x86_64-unknown-linux-gnu\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}
