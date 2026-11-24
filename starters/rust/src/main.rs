use review_triage::{Input, ReviewResult, Task, validate};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const DEFAULT_TEMPLATE: &str = "templates/customer-review-triage";

#[derive(Deserialize)]
struct Expected {
    result: ReviewResult,
    issue_counts: BTreeMap<String, usize>,
}

#[derive(Deserialize)]
struct ValidationCase {
    id: String,
    expected_error: String,
    input: Option<Input>,
    result: Value,
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let template = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| DEFAULT_TEMPLATE.into());
    let candidate = args.next().map(PathBuf::from);
    if args.next().is_some() {
        return Err("Usage: review-triage [template-directory] [candidate-result.json]".into());
    }

    let task: Task = read_json(&template.join("task.json"))?;
    let input: Input = read_json(&template.join("sample-input.json"))?;
    let (result, counts) = if let Some(path) = candidate {
        let result: ReviewResult = read_json(&path)?;
        let counts = validate(&task, &input, &result)?;
        (result, counts)
    } else {
        let expected: Expected = read_json(&template.join("expected-output.json"))?;
        let counts = validate(&task, &input, &expected.result)?;
        if counts != expected.issue_counts {
            return Err("Fixture counts do not match expected-output.json".into());
        }
        let cases: Vec<ValidationCase> = read_json(&template.join("validation-cases.json"))?;
        for case in cases {
            let outcome = serde_json::from_value(case.result)
                .map_err(|_| "invalid_shape")
                .and_then(|result| validate(&task, case.input.as_ref().unwrap_or(&input), &result));
            if outcome.err() != Some(case.expected_error.as_str()) {
                return Err(format!("Validation case failed: {}", case.id));
            }
        }
        (expected.result, counts)
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"result": result, "issue_counts": counts}))
            .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
