# LLM starter template use cases

Method on 2026-11-23 ICT (UTC+07:00). These are hypothetical workflows. The sample outputs were worked out by hand from synthetic inputs. User demand, model quality, and time savings require live tests.

## Simulated workflows

| User and recurring job | Input | Useful output | Useful template defaults |
| --- | --- | --- | --- |
| Online seller reviews customer complaints each week | A batch of review text with record IDs | Issue summary, unique review count per issue, and source excerpts | Fixed categories, multiple labels per item, evidence checking, and CSV export after review |
| Support lead drafts answers for repeated questions | Incoming ticket and the relevant policy excerpt | Editable draft with the policy reference and missing facts to ask about | Approved source selection and human review before sending |
| Project coordinator turns notes into next steps | Meeting notes or a transcript | Action-item table with owner, due date, and supporting quote | Nullable fields and a queue for ambiguous or unassigned tasks |
| Operations assistant copies information from documents | Text extracted from invoices or forms | Draft rows for a spreadsheet with field-level evidence | Typed schema, required-field validation, and a human review step |
| Analyst checks a factual question against public sources | A question and a preferred source type | Short answer with clickable citations | Source requirements, citations beside claims, and a clear outcome when evidence is missing |

## Worked simulation for customer reviews

Four synthetic records exercise multi-label classification, evidence, and ambiguity.

| Review ID | Text |
| --- | --- |
| R1 | Delivery took five days longer than expected. The product works fine. |
| R2 | The box arrived crushed, but the item still works. |
| R3 | It arrived late and the box was torn. |
| R4 | Fine, I guess. |

Expected outcome from a manual pass.

| Issue | Unique reviews | Supporting record IDs |
| --- | --- | --- |
| Late delivery | 2 | R1, R3 |
| Damaged packaging | 2 | R2, R3 |

R3 contributes once to each issue. R4 stays unclassified and is available for human review. The app should show the exact excerpts with their IDs. Rust can validate the IDs and excerpts and compute unique counts.

## First template to prototype

**Extract evidence-backed rows from a batch of text.** Use customer reviews as the example, then try meeting notes by changing the output fields and validation rules.

| Reusable template element | First prototype |
| --- | --- |
| Input | A synthetic batch with stable record IDs and selectable categories |
| Output | Rows with category, record ID, and exact supporting excerpt, plus an unclassified list |
| Rules | Allow multiple categories, preserve ambiguous records, and require a valid ID and excerpt for every row |
| Checks | Test multi-label input, empty input, invalid IDs, duplicate records, unsupported claims, and ambiguous text |
| Review and export | Let users inspect and edit rows before exporting CSV |

A developer starting with the template should be able to run a provided synthetic example, inspect the expected output, then change the schema and rules for another job. Keep account connection, progress display, and request handling in the shared Rust app.

### Customer review template files

- [task.json](../templates/customer-review-triage/task.json) defines the template. Edit `categories` to change the issue labels. Edit `input_schema` and `output_schema` to change the accepted records and returned fields. Update `validation_rules` for cross-record checks and `review.editable_row_fields` for the fields a reviewer can correct.
- [sample-input.json](../templates/customer-review-triage/sample-input.json) contains the four synthetic reviews. Keep IDs unique when adding examples.
- [expected-output.json](../templates/customer-review-triage/expected-output.json) contains the manually prepared result and reference counts. Update this file when the fixture changes. The counts are computed from unique review IDs per category.

The fixture check verified the structure, exact excerpts, counts, and treatment of the ambiguous review. The Rust implementation and provider test are tracked in [plan.md](../plan.md).

### Cross-language fixture results

The Rust, Python, and TypeScript starters returned matching reference rows and counts on 2026-11-23 ICT (UTC+07:00). Each starter verified all nine [shared rejection cases](../templates/customer-review-triage/validation-cases.json). Each accepted the valid candidate with reversed row order and rejected a candidate with an excerpt absent from its source. These runs validate the fixture and rules locally. Review integration in the web app and live model output remain pending.

### Rust web integration

The web app consumes the shared JSON template and the Rust starter validator. The local build and seven Rust tests passed. Browser checks opened the sample, rejected an edited excerpt absent from R1, showed the row error, and enabled export after correction. HTTP checks exercised the nine shared rejection cases and verified the corrected CSV. The sample remained at two unique reviews in each issue category with R4 unclassified.

New review batches use a structured response on the direct ChatGPT plan path. See the live test below for the selected model's response and attribution status. OpenAI documents the Responses API structured text format and the separate plan-usage preview requirements. Documentation checked on 2026-11-23 11:49 ICT (UTC+07:00). [Structured outputs — OpenAI](https://developers.openai.com/api/docs/guides/structured-outputs) · [Plan usage preview limitations — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)

### Live ChatGPT review test

On 2026-11-24 ICT (UTC+07:00), the Rust web app used the connected ChatGPT plan flow and sent the four synthetic reviews to `gpt-6.2-sol`. The app displayed response `resp_04a643e0…7d48b`.

| Check | Observed result |
| --- | --- |
| Late delivery | 2 unique reviews, R1 and R3 |
| Damaged packaging | 2 unique reviews, R2 and R3 |
| Unclassified | R4 |
| Evidence | Four excerpts matched their source reviews |
| Export validation | Replacing the first excerpt with invented text highlighted row 1 and disabled CSV export. Restoring the excerpt enabled export. |

The app reported an active ChatGPT plan connection. Usage attribution for this request remains unverified in ChatGPT Settings.

## Next validation

Ask a newcomer to run the demo or starter from the README and explain what the results establish. Ask a developer to adapt the example to a task of their choice and record where they needed help with configuration, schema, fixtures, or validation. Use the feedback to improve how the project teaches and supports reuse.
