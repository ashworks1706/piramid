//! Loading a tokenizer.json tokenizer from a checkpoint directory.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::path::Path;

use piramid_model::inference::backends::tokenizers::JsonTokenizer;

#[test]
fn a_tokenizer_path_that_is_a_file_is_refused() {
    let file = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let error = JsonTokenizer::load(file).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("must be a directory holding tokenizer.json"),
        "{error}"
    );
}
