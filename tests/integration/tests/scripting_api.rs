//! Keeps the checked-in scripting API manifest honest: it must always match
//! what `aigs_runtime::api_manifest()` actually reports (which in turn is
//! smoke-tested against the real rhai registrations in `aigs-runtime`).

use std::path::Path;

#[test]
fn scripting_api_manifest_matches_checked_in_json() {
    let generated = serde_json::to_string_pretty(&aigs_runtime::api_manifest())
        .expect("manifest must serialize");
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sdk/aigs-format/scripting-api.json");
    let checked_in =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing {}", path.display()));
    // Normalize line endings: a Windows checkout may turn the checked-in
    // file's LF into CRLF, while `to_string_pretty` always emits LF.
    let normalize = |s: &str| s.replace("\r\n", "\n").trim().to_string();
    assert_eq!(
        normalize(&generated),
        normalize(&checked_in),
        "sdk/aigs-format/scripting-api.json is stale — regenerate with \
         `cargo run -p aigs-cli -- script-api > sdk/aigs-format/scripting-api.json`"
    );
}
