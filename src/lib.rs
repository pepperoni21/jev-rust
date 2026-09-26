//! A simple, type-safe async client for the [TypeSafe](https://docs.typesafe.ai)
//! evaluation API.
//!
//! Evaluate a `state` against a map of typed questions and get back structured
//! answers, one per question.

mod client;
mod error;
mod response;
mod types;

pub use client::{RetryPolicy, TypeSafe};
pub use error::Error;
pub use response::{Answer, Answers, ChoiceAnswer, EvaluateResponse, ScoreAnswer, Usage};
pub use types::{Choice, EvaluateRequest, Noul, NoulCriteria, Question, Score, State};
