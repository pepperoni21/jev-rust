//! Request types for building an evaluation.

use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Convert any serializable value into a `serde_json::Value`.
///
/// Used to accept flexible `state`, `instructions`, and `criteria` inputs.
/// Serialization failures (which do not occur for well-formed data) fall back
/// to JSON `null`.
fn to_value<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// The content to evaluate: a plain string, an array of text values, or a
/// structured JSON object.
///
/// Construct one with [`State::text`], [`State::texts`], or [`State::object`].
/// [`EvaluateRequest::new`] also accepts plain strings directly.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum State {
    /// A single message, article, or passage.
    Text(String),
    /// An ordered sequence of text values (e.g. a list of messages).
    TextList(Vec<String>),
    /// A structured object with named fields (e.g. records or application
    /// state).
    Object(Value),
}

impl State {
    /// A plain string state.
    pub fn text(text: impl Into<String>) -> Self {
        State::Text(text.into())
    }

    /// An ordered list of text values (e.g. a sequence of messages).
    pub fn texts<I, S>(list: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        State::TextList(list.into_iter().map(Into::into).collect())
    }

    /// A structured JSON object state.
    ///
    /// Accepts anything serializable, e.g. `serde_json::json!({...})` or a
    /// struct deriving `Serialize`. It should serialize to a JSON object.
    pub fn object(value: impl Serialize) -> Self {
        State::Object(to_value(value))
    }
}

impl From<String> for State {
    fn from(text: String) -> Self {
        State::Text(text)
    }
}

impl From<&str> for State {
    fn from(text: &str) -> Self {
        State::Text(text.to_string())
    }
}

impl From<Vec<String>> for State {
    fn from(list: Vec<String>) -> Self {
        State::TextList(list)
    }
}

/// The top-level request sent to the evaluation endpoint.
///
/// Build one with [`EvaluateRequest::new`] and add typed questions with
/// [`EvaluateRequest::question`].
#[derive(Debug, Clone, Serialize)]
pub struct EvaluateRequest {
    state: State,
    model: String,
    questions: BTreeMap<String, Question>,
}

impl EvaluateRequest {
    /// Start building a request from the content to evaluate.
    ///
    /// `state` can be a plain string, an array of text values, or a JSON
    /// object. Pass a `&str` or `String` directly, or one of the [`State`]
    /// constructors for structured content.
    pub fn new(state: impl Into<State>) -> Self {
        Self {
            state: state.into(),
            model: "jev-latest".to_string(),
            questions: BTreeMap::new(),
        }
    }

    /// Set the model that handles the request (defaults to `"jev-latest"`).
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Add a typed question under the given id. Answers are returned under the
    /// same id.
    pub fn question(mut self, id: impl Into<String>, question: impl Into<Question>) -> Self {
        self.questions.insert(id.into(), question.into());
        self
    }
}

/// A typed question. One of [`Noul`], [`Choice`], or [`Score`].
///
/// You normally construct this via the builder types and their `From` impls,
/// rather than directly.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    Noul {
        instructions: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        criteria: Option<NoulCriteria>,
    },
    Choice {
        instructions: Value,
        criteria: BTreeMap<String, Value>,
    },
    Score {
        instructions: Value,
        criteria: Vec<Value>,
    },
}

/// Optional descriptions of what a "yes" and a "no" mean for a [`Noul`] question.
#[derive(Debug, Clone, Default, Serialize)]
pub struct NoulCriteria {
    #[serde(rename = "true", skip_serializing_if = "Option::is_none")]
    yes: Option<Value>,
    #[serde(rename = "false", skip_serializing_if = "Option::is_none")]
    no: Option<Value>,
}

/// A yes/no question. Returns the probability the answer is "yes".
#[derive(Debug, Clone)]
pub struct Noul {
    instructions: Value,
    criteria: NoulCriteria,
}

impl Noul {
    /// Create a yes/no question from its instructions.
    pub fn new(instructions: impl Serialize) -> Self {
        Self {
            instructions: to_value(instructions),
            criteria: NoulCriteria::default(),
        }
    }

    /// Describe what a "yes" (value near 1) means.
    pub fn yes(mut self, description: impl Serialize) -> Self {
        self.criteria.yes = Some(to_value(description));
        self
    }

    /// Describe what a "no" (value near 0) means.
    pub fn no(mut self, description: impl Serialize) -> Self {
        self.criteria.no = Some(to_value(description));
        self
    }

    /// Set both the "yes" and "no" descriptions at once.
    pub fn criteria(mut self, yes: impl Serialize, no: impl Serialize) -> Self {
        self.criteria.yes = Some(to_value(yes));
        self.criteria.no = Some(to_value(no));
        self
    }
}

impl From<Noul> for Question {
    fn from(n: Noul) -> Self {
        let criteria = if n.criteria.yes.is_some() || n.criteria.no.is_some() {
            Some(n.criteria)
        } else {
            None
        };
        Question::Noul {
            instructions: n.instructions,
            criteria,
        }
    }
}

/// A multiple-choice question. Returns the chosen option and the full
/// probability distribution.
#[derive(Debug, Clone)]
pub struct Choice {
    instructions: Value,
    criteria: BTreeMap<String, Value>,
}

impl Choice {
    /// Create a choice question from its instructions.
    pub fn new(instructions: impl Serialize) -> Self {
        Self {
            instructions: to_value(instructions),
            criteria: BTreeMap::new(),
        }
    }

    /// Add an option with a description of what it means.
    pub fn option(mut self, name: impl Into<String>, description: impl Serialize) -> Self {
        self.criteria.insert(name.into(), to_value(description));
        self
    }

    /// Add an option that needs no extra description (sent as JSON `null`).
    pub fn option_none(mut self, name: impl Into<String>) -> Self {
        self.criteria.insert(name.into(), Value::Null);
        self
    }
}

impl From<Choice> for Question {
    fn from(c: Choice) -> Self {
        Question::Choice {
            instructions: c.instructions,
            criteria: c.criteria,
        }
    }
}

/// A score question. Rates the state along an ordered rubric and returns a
/// probability-weighted value across the levels.
#[derive(Debug, Clone)]
pub struct Score {
    instructions: Value,
    criteria: Vec<Value>,
}

impl Score {
    /// Create a score question from its instructions.
    pub fn new(instructions: impl Serialize) -> Self {
        Self {
            instructions: to_value(instructions),
            criteria: Vec::new(),
        }
    }

    /// Append one level description (in order).
    pub fn level(mut self, description: impl Serialize) -> Self {
        self.criteria.push(to_value(description));
        self
    }

    /// Append several level descriptions in order.
    pub fn levels<I, T>(mut self, descriptions: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Serialize,
    {
        self.criteria
            .extend(descriptions.into_iter().map(|d| to_value(d)));
        self
    }
}

impl From<Score> for Question {
    fn from(s: Score) -> Self {
        Question::Score {
            instructions: s.instructions,
            criteria: s.criteria,
        }
    }
}
