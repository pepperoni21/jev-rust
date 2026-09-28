use jev_rust::{Answer, Choice, EvaluateRequest, EvaluateResponse, Noul, Score, State, TypeSafe};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Reads the API key from the TYPESAFE_API_KEY environment variable
    // Alternatively you can use TypeSafe::new("your-api-key")
    let typesafe = TypeSafe::from_env()?;

    // A plain string state
    let text = EvaluateRequest::new("Help! My payouts have been failing for 3 days.")
        .question("is_urgent", Noul::new("Does this convey urgency?"));

    // A structured object state
    let object = EvaluateRequest::new(State::object(json!({
        "message": "My card was charged twice.",
        "order_id": "A-104",
    })))
    .question(
        "department",
        Choice::new("Which team should handle this?")
            .option("billing", "Payments, invoicing, refunds")
            .option("technical", "Bugs, outages, integrations"),
    )
    .question(
        "frustration",
        Score::new("How frustrated is the customer?").levels(["Calm", "Frustrated", "Very angry"]),
    );

    // An array of text values
    let messages = EvaluateRequest::new(State::texts([
        "Hi",
        "My customer number is TS1337.",
        "My card was charged twice.",
    ]))
    .question(
        "mentions_charge",
        Noul::new("Does the customer mention a charge?"),
    );

    for (label, request) in [("text", text), ("object", object), ("messages", messages)] {
        let response = typesafe.evaluate(request).await?;
        print_answers(label, &response);
    }

    Ok(())
}

/// Print an [`EvaluateResponse`] in a human-readable form.
fn print_answers(label: &str, response: &EvaluateResponse) {
    println!("=== {label} ===");
    println!("model: {}", response.model);
    println!(
        "tokens: {} in / {} out",
        response.usage.input_tokens, response.usage.output_tokens
    );

    for (id, answer) in response.answers.iter() {
        match answer {
            Answer::Noul { noul } => {
                println!("  {id}: {:.1}% yes", noul * 100.0);
            }
            Answer::Choice {
                choice,
                probabilities,
                confidence,
            } => {
                println!("  {id}: {choice} ({:.1}% confident)", confidence * 100.0);
                for (option, probability) in probabilities {
                    println!("      - {option}: {:.1}%", probability * 100.0);
                }
            }
            Answer::Score {
                score,
                legend,
                probabilities,
                confidence,
            } => {
                println!("  {id}: {score:.2} ({:.1}% confident)", confidence * 100.0);
                for (level, probability) in probabilities {
                    let description = legend.get(level).map(String::as_str).unwrap_or(level);
                    println!("      - {description}: {:.1}%", probability * 100.0);
                }
            }
        }
    }
    println!();
}
