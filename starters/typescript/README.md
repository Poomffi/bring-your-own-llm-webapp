# TypeScript starter

The reference fixture and all nine shared rejection cases pass locally without a provider account. This starter is a local Node.js CLI example.

With Node.js 24 or newer, run from the repository root.

```text
node starters/typescript/main.ts
node starters/typescript/main.ts templates/customer-review-triage candidate.json
```

The default run reads the shared synthetic input, validates the reference result and rejection cases, and prints the expected rows with recomputed issue counts. The same command can validate a candidate result supplied as a second argument. Pass a JSON object with `rows` and `unclassified_review_ids` using the shape in [task.json](../../templates/customer-review-triage/task.json). Use the `result` value inside the reference file as an example.

To adapt the starter to another set of review categories, copy the template folder and update the categories and fixtures together. A different input or output shape requires changing the validator for that task.
