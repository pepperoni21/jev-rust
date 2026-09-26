//! Response types returned by the evaluation endpoint.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::ops::Deref;

/// The top-level response from an evaluation.
#[derive(Debug, Clone, Deserialize)]
pub struct EvaluateResponse {
    /// The model that performed the evaluation.
    pub model: String,
    /// One answer per question, keyed by the ids used in the request.
    pub answers: Answers,
    /// Token usage for the request.
    pub usage: Usage,
}

/// Token usage for a request.
#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// The collection of answers for a request, keyed by question id.
///
/// Wraps the underlying map and adds typed accessors so you don't have to
/// match on [`Answer`] yourself. It derefs to `BTreeMap<String, Answer>`, so
/// ordinary map operations (`iter`, `len`, `contains_key`, indexing, etc.) are
/// available too.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct Answers(BTreeMap<String, Answer>);

impl Answers {
    /// Returns the raw answer for a question id.
    pub fn get(&self, id: &str) -> Option<&Answer> {
        self.0.get(id)
    }

    /// Returns a noul answer as a probability of "yes" (0..1).
    pub fn noul(&self, id: &str) -> Option<f64> {
        self.0.get(id).and_then(Answer::noul)
    }

    /// Returns a choice answer (chosen option + full distribution).
    pub fn choice(&self, id: &str) -> Option<ChoiceAnswer> {
        self.0.get(id).and_then(Answer::choice)
    }

    /// Returns a score answer (weighted value + distribution).
    pub fn score(&self, id: &str) -> Option<ScoreAnswer> {
        self.0.get(id).and_then(Answer::score)
    }
}

impl Deref for Answers {
    type Target = BTreeMap<String, Answer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An answer to a single question, discriminated by its `type`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    Noul {
        noul: f64,
    },
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
    Score {
        score: f64,
        legend: BTreeMap<String, String>,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
}

/// A choice answer: the chosen option and the full probability distribution.
#[derive(Debug, Clone)]
pub struct ChoiceAnswer {
    /// The highest-probability option.
    pub choice: String,
    /// Every option mapped to its probability.
    pub probabilities: BTreeMap<String, f64>,
    /// How certain the model is, derived from the probabilities.
    pub confidence: f64,
}

/// A score answer: the probability-weighted value and its distribution.
#[derive(Debug, Clone)]
pub struct ScoreAnswer {
    /// The probability-weighted answer across the levels.
    pub score: f64,
    /// Each level number mapped back to its description.
    pub legend: BTreeMap<String, String>,
    /// Each level mapped to its probability.
    pub probabilities: BTreeMap<String, f64>,
    /// How certain the model is, derived from the probabilities.
    pub confidence: f64,
}

impl Answer {
    /// If this is a noul answer, returns the probability of "yes" (0..1).
    pub fn noul(&self) -> Option<f64> {
        match self {
            Self::Noul { noul } => Some(*noul),
            _ => None,
        }
    }

    /// If this is a choice answer, returns the chosen option and distribution.
    pub fn choice(&self) -> Option<ChoiceAnswer> {
        match self {
            Self::Choice {
                choice,
                probabilities,
                confidence,
            } => Some(ChoiceAnswer {
                choice: choice.clone(),
                probabilities: probabilities.clone(),
                confidence: *confidence,
            }),
            _ => None,
        }
    }

    /// If this is a score answer, returns the weighted score and distribution.
    pub fn score(&self) -> Option<ScoreAnswer> {
        match self {
            Self::Score {
                score,
                legend,
                probabilities,
                confidence,
            } => Some(ScoreAnswer {
                score: *score,
                legend: legend.clone(),
                probabilities: probabilities.clone(),
                confidence: *confidence,
            }),
            _ => None,
        }
    }
}
