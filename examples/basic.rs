use jev_rust::{Choice, EvaluateRequest, Noul, Score, State, TypeSafe};
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
        println!("--- {label} ---");
        println!("model: {}", response.model);
        println!("answers: {:#?}", response.answers);
    }

    Ok(())
}
