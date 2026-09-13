//! Stop strings over streamed text: text that could begin a stop string is held back until it
//! either completes one or cannot.

/// Watches streamed text for any of a set of stop strings.
#[derive(Debug, Clone, Default)]
pub struct StopMatcher {
    stops: Vec<String>,
    held: String,
}

/// What one push of text produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopOutcome {
    /// Text safe to release.
    pub text: String,
    /// Whether a stop string was completed; text ends before it.
    pub stopped: bool,
}

impl StopMatcher {
    /// A matcher for the given stop strings. Empty strings are ignored.
    pub fn new(stops: &[String]) -> Self {
        Self {
            stops: stops.iter().filter(|s| !s.is_empty()).cloned().collect(),
            held: String::new(),
        }
    }

    /// Add streamed text.
    pub fn push(&mut self, delta: &str) -> StopOutcome {
        self.held.push_str(delta);
        let first_match = self
            .stops
            .iter()
            .filter_map(|stop| self.held.find(stop.as_str()))
            .min();
        if let Some(position) = first_match {
            let text = self.held[..position].to_string();
            self.held.clear();
            return StopOutcome {
                text,
                stopped: true,
            };
        }
        let keep = self
            .stops
            .iter()
            .map(|stop| longest_prefix_suffix(&self.held, stop))
            .max()
            .unwrap_or(0);
        let release = self.held.len() - keep;
        let text = self.held[..release].to_string();
        self.held.drain(..release);
        StopOutcome {
            text,
            stopped: false,
        }
    }

    /// Release any text still held, at the end of a generation.
    pub fn flush(&mut self) -> String {
        std::mem::take(&mut self.held)
    }
}

/// Length of the longest suffix of text that is a proper prefix of stop, on a char boundary.
fn longest_prefix_suffix(text: &str, stop: &str) -> usize {
    let max = stop.len().saturating_sub(1).min(text.len());
    (1..=max)
        .rev()
        .find(|&len| {
            let start = text.len() - len;
            text.is_char_boundary(start) && stop.starts_with(&text[start..])
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
