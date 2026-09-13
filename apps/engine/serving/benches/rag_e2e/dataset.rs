//! The question file of the end-to-end benchmark: one JSON question per line.

use std::collections::HashSet;

use serde::Deserialize;

/// One passage a question is retrieved against.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    /// Identifier, shared by every question that lists the same passage.
    pub id: String,
    /// Text stored and placed before the prompt.
    pub text: String,
    /// Whether the passage supports the answer.
    pub gold: bool,
}

/// One question with its accepted answers and candidate passages.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Identifier, unique within the file.
    pub id: String,
    /// The question asked of the model.
    pub question: String,
    /// Accepted answers. Any one of them counts as a match.
    pub answers: Vec<String>,
    /// Candidate passages, gold and distractor.
    pub passages: Vec<Passage>,
}

impl Question {
    /// Ids of the passages marked gold.
    pub fn gold_ids(&self) -> Vec<&str> {
        self.passages
            .iter()
            .filter(|passage| passage.gold)
            .map(|passage| passage.id.as_str())
            .collect()
    }
}

/// Parse one line of the file. Number is the one-based line number used in errors.
pub fn parse_line(line: &str, number: usize) -> Result<Question, String> {
    let question: Question =
        serde_json::from_str(line).map_err(|e| format!("dataset line {number}: {e}"))?;
    if question.id.is_empty() {
        return Err(format!("dataset line {number}: id is empty"));
    }
    if question.question.trim().is_empty() {
        return Err(format!("dataset line {number}: question is empty"));
    }
    if question
        .answers
        .iter()
        .all(|answer| answer.trim().is_empty())
    {
        return Err(format!("dataset line {number}: answers holds no answer"));
    }
    if question.passages.is_empty() {
        return Err(format!("dataset line {number}: passages is empty"));
    }
    if let Some(passage) = question
        .passages
        .iter()
        .find(|passage| passage.id.is_empty())
    {
        return Err(format!(
            "dataset line {number}: a passage has an empty id and text {:?}",
            passage.text
        ));
    }
    Ok(question)
}

/// Parse a whole file, skipping blank lines and keeping at most limit questions.
pub fn parse(contents: &str, limit: Option<usize>) -> Result<Vec<Question>, String> {
    let mut questions = Vec::new();
    let mut seen = HashSet::new();
    for (index, line) in contents.lines().enumerate() {
        if limit.is_some_and(|limit| questions.len() >= limit) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let question = parse_line(line, index + 1)?;
        if !seen.insert(question.id.clone()) {
            return Err(format!(
                "dataset line {}: question id {} appears more than once",
                index + 1,
                question.id
            ));
        }
        questions.push(question);
    }
    if questions.is_empty() {
        return Err("the dataset holds no question".to_string());
    }
    Ok(questions)
}

/// Every distinct passage across questions, in first-seen order. The first text of an id wins.
pub fn unique_passages(questions: &[Question]) -> Vec<&Passage> {
    let mut seen = HashSet::new();
    questions
        .iter()
        .flat_map(|question| question.passages.iter())
        .filter(|passage| seen.insert(passage.id.as_str()))
        .collect()
}
