# Python starter

The reference fixture and all nine shared rejection cases pass locally without a provider account. This starter is a local CLI example.

With Python 3, run from the repository root.

```text
python starters/python/main.py
python starters/python/main.py templates/customer-review-triage candidate.json
```

The default run reads the shared synthetic input, validates the reference result and rejection cases, and prints the expected rows with recomputed issue counts. The same command can validate a candidate result supplied as a second argument. Pass a JSON object with `rows` and `unclassified_review_ids` using the shape in [task.json](../../templates/customer-review-triage/task.json). Use the `result` value inside the reference file as an example.

To adapt the starter to another set of review categories, copy the template folder and update the categories and fixtures together. A different input or output shape requires changing the validator for that task.
