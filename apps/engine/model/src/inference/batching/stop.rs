//! Stop strings over streamed text: a possible prefix is held back until it resolves.

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
    /// A matcher for the given stop strings.
    pub fn new(stops: &[String]) -> Self {
        Self {
            stops: stops.to_vec(),
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
            self.held.truncate(position);
            return StopOutcome {
                text: std::mem::take(&mut self.held),
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
        StopOutcome {
            text: self.held.drain(..release).collect(),
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
