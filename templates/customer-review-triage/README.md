# Customer-review triage

Convert review text into classified rows with a category ID, source review ID, and exact excerpt. Keep ambiguous reviews in the unclassified list.

| File | Contents |
| --- | --- |
| [task.json](task.json) | Categories, input and output schemas, and validation rules. |
| [sample-input.json](sample-input.json) | Four synthetic reviews and their stable IDs. |
| [expected-output.json](expected-output.json) | Reference classifications, excerpts, and unique review counts. |
| [validation-cases.json](validation-cases.json) | Shared cases the validators must reject. |

The fixtures passed in Rust, Python, and TypeScript. A live ChatGPT run returned two late-delivery reviews and two damaged-packaging reviews. R3 counted once in each category and R4 remained unclassified.

Run the fixture with a [language starter](../../starters/README.md) or select **Open review demo** in the [web app](../../README.md#run-the-local-web-app). When adapting the example, update the categories and fixtures together. Changes to the data shape require matching validator changes.
