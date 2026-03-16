use std::fs;

#[test]
fn cargo_fuzz_gate_runs_from_fuzz_directory() {
    let script =
        fs::read_to_string("scripts/check-fuzz.sh").expect("fuzz gate script should be readable");

    assert!(
        script.contains("pushd \"$ROOT/fuzz\" >/dev/null"),
        "cargo-fuzz should run from fuzz/ so the pinned nightly toolchain applies"
    );
}
