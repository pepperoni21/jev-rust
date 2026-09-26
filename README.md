# jev-rust

A small, type-safe, async client for the [TypeSafe](https://docs.typesafe.ai) System One evaluation API.

## Installation

```toml
[dependencies]
jev-rust = "0.1"
```

The client is built on `tokio`, `reqwest`, and `serde`, and is fully `async`.

## Quick start

```rust
use jev_rust::{Choice, EvaluateRequest, Noul, Score, TypeSafe};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads the API key from the TYPESAFE_API_KEY environment variable.
    // Use `TypeSafe::new("your-api-key")` to pass it explicitly.
    let client = TypeSafe::from_env()?;

    let request = EvaluateRequest::new(json!({
        "message": "Help! My payouts have been failing for 3 days.",
        "order_id": "A-104",
    }))
    .question("is_urgent", Noul::new("Does this convey urgency?"))
    .question("department", Choice::new("Which team should handle this?")
        .option("billing", "Payments, invoicing, refunds")
        .option("technical", "Bugs, outages, integrations"))
    .question("frustration", Score::new("How frustrated is the customer?")
        .levels(["Calm", "Frustrated", "Very angry"]));

    let response = client.evaluate(request).await?;
    println!("{:#?}", response.answers);

    Ok(())
}
```

## State

`state` is the content to evaluate. It can be one of three shapes:

```rust
use jev_rust::{EvaluateRequest, State};
use serde_json::json;

// A plain string
let a = EvaluateRequest::new("My card was charged twice.");

// A JSON object (recommended for most requests)
let b = EvaluateRequest::new(State::object(json!({
    "message": "My card was charged twice.",
    "order_id": "A-104",
})));

// An ordered list of text values (e.g. a message sequence)
let c = EvaluateRequest::new(State::texts(["Hi", "My card was charged twice."]));
```

`State::object` accepts anything that implements `Serialize`, so you can pass a struct too:

```rust
#[derive(serde::Serialize)]
struct Ticket { message: String, order_id: String }

let request = EvaluateRequest::new(State::object(Ticket {
    message: "My card was charged twice.".into(),
    order_id: "A-104".into(),
}));
```

## Questions

Add questions to a request with `.question(id, question)`. Answers come back under the same id. There are three question types:

| Type     | Builder | Returns                    |
| -------- | ------- | -------------------------- |
| yes/no   | `Noul`  | probability of "yes" (0–1) |
| choice   | `Choice`| chosen option + distribution |
| score    | `Score` | probability-weighted value |

```rust
Noul::new("Does this convey urgency?")
    .yes("Explicitly time-sensitive")
    .no("No urgency expressed");

Choice::new("Which team should handle this?")
    .option("billing", "Payments, invoicing, refunds")
    .option("technical", "Bugs, outages, integrations");

Score::new("How frustrated is the customer?")
    .levels(["Calm", "Frustrated", "Very angry"]);
```

## Reading answers

`response.answers` is an `Answers` collection keyed by question id, with typed accessors:

```rust
let response = client.evaluate(request).await?;

// yes/no -> Option<f64>
if let Some(urgent) = response.answers.noul("is_urgent") {
    println!("urgency: {urgent:.2}");
}

// choice -> Option<ChoiceAnswer>
if let Some(department) = response.answers.choice("department") {
    println!("{} (confidence {:.2})", department.choice, department.confidence);
}

// score -> Option<ScoreAnswer>
if let Some(frustration) = response.answers.score("frustration") {
    println!("frustration: {:.2}", frustration.score);
}
```

`Answers` also derefs to `BTreeMap<String, Answer>`, so ordinary map operations (`iter`, `len`, `get`, …) work, and you can match on the raw `Answer` enum if you prefer.

## Configuration

```rust
use jev_rust::{RetryPolicy, TypeSafe};

// Explicit API key
let client = TypeSafe::new("your-api-key");

// Or from TYPESAFE_API_KEY
let client = TypeSafe::from_env()?;

// Custom base URL (proxy / self-hosted)
let client = TypeSafe::new("your-api-key").base_url("https://example.com");

// Retry policy (default: exponential, up to 3 retries, for 429/529)
let client = TypeSafe::new("your-api-key").no_retries();
let client = TypeSafe::new("your-api-key").retry(RetryPolicy::exponential(5));
```

## Errors

`evaluate` returns a `jev_rust::Error`. Transient `429 Too Many Requests` and `529 Overloaded` responses are retried with exponential backoff by default; other errors (`401`, `422`, etc.) are returned immediately.
