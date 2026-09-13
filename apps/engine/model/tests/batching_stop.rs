//! Stop strings held back and matched across streamed text deltas.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_model::inference::batching::StopMatcher;

fn stops(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_stop_string_split_across_deltas_is_caught_and_not_released() {
    let mut matcher = StopMatcher::new(&stops(&["</answer>"]));
    let mut released = String::new();
    let mut stopped = false;
    for delta in ["Paris", " is it</", "ans", "wer> trailing"] {
        let outcome = matcher.push(delta);
        released.push_str(&outcome.text);
        if outcome.stopped {
            stopped = true;
            break;
        }
    }
    assert!(stopped);
    assert_eq!(released, "Paris is it");
}

#[test]
fn held_text_is_released_once_it_cannot_start_a_stop() {
    let mut matcher = StopMatcher::new(&stops(&["\n\n"]));
    assert_eq!(matcher.push("a\n").text, "a");
    assert_eq!(matcher.push("b").text, "\nb");
    assert_eq!(matcher.push("c\n").text, "c");
    assert_eq!(matcher.flush(), "\n");
}

#[test]
fn without_stops_everything_is_released() {
    let mut matcher = StopMatcher::new(&[]);
    let outcome = matcher.push("anything");
    assert_eq!(outcome.text, "anything");
    assert!(!outcome.stopped);
}
