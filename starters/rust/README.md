# Rust starter

The reference fixture and all nine shared rejection cases pass locally without a provider account. The web app uses this validator for review checks and CSV export.

With a Rust toolchain that supports edition 2024, run from the repository root.

```text
cargo run --locked --manifest-path starters/rust/Cargo.toml
cargo run --locked --manifest-path starters/rust/Cargo.toml -- templates/customer-review-triage candidate.json
```

The default run reads the shared synthetic input, validates the reference result and rejection cases, and prints the expected rows with recomputed issue counts. The same command can validate a candidate result supplied as a second argument. Pass a JSON object with `rows` and `unclassified_review_ids` using the shape in [task.json](../../templates/customer-review-triage/task.json). Use the `result` value inside the reference file as an example.

To adapt the starter to another set of review categories, copy the template folder and update the categories and fixtures together. A different input or output shape requires changing the validator for that task.

The library exposes `validate` so a Rust application can check the result before showing or exporting it.
