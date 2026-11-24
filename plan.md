# Bring Your Own LLM Web App

## Goal

Give developers and anyone exploring LLM apps a runnable way to see what an integration can do, understand its limits, and adapt the examples to their own work. Verify direct provider authorization and inference, and record which existing plan or organization covered usage when the evidence supports it.

## Scope

Start with ChatGPT plan usage for a local prototype. Evaluate Claude Platform Linked Billing if billing an existing Claude API organization fits the intended users. Study a direct provider API key as a fallback with its own API billing and compare MCP when analysis happens in the AI host. The OpenRouter experiment established the local OAuth scaffold. Paid inference through that path would draw from OpenRouter credits.

## First milestone — Verify direct ChatGPT plan usage

Documented by OpenAI and checked on 2026-11-21 14:38 ICT (UTC+07:00). Eligible ChatGPT Plus and Pro users can grant plan usage separately from identity. The documented local path is available for open-source and local personal projects. Paid or remotely hosted apps can request access through the waitlist. Project and account eligibility remain pending. [Quickstart](https://developers.openai.com/siwc/quickstart) · [Usage policy and terms](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt#usage-policy-and-terms)

1. Confirm that the local Rust client and a ChatGPT Plus or Pro test account qualify for the documented Sign in with ChatGPT plan-usage flow. Check hosted-app access separately before planning a public deployment. Record any approval or account requirement.
2. Implement the provider's published authorization flow. Confirm that the account grants inference permission before enabling **Analyze with AI**.
3. Send one synthetic text request through the supported Responses API and display the result in the app. Compare the connected account's app usage in ChatGPT settings before and after the request. Record any attribution that the available evidence cannot establish.
4. Verify that disconnect clears saved credentials and that a new request requires reconnecting. Record consent, inference, plan usage, and disconnect as separate results.
5. If eligibility blocks the test, document the specific restriction. Evaluate Claude Linked Billing for users with an eligible Claude Platform organization and direct BYOK for users with their own provider API billing.

## Next milestone — Choose how to connect

Start on a connection chooser. After connecting to a provider, show its supported task and model controls. The demo opens the prepared review result. Show the active provider and where usage is counted. Provide a way to return to the chooser and clear the active connection.

| Entry | First option | Usage source |
| --- | --- | --- |
| Sign in with an AI account | ChatGPT plan usage for eligible Plus or Pro accounts | The connected ChatGPT plan when authorized for inference |
| Use an API key | Gemini Developer API Free tier | The quota or billing of the project that issued the key |
| Run a local model | Downloaded model served by Ollama on the same computer | Local machine resources |
| Try a demo | Manually prepared review fixture | The local app shows the fixture without a provider request |

Keep the sign-in list limited to integrations with a documented authorization path for inference. Evaluate Claude Platform Linked Billing separately for eligible API organizations. The organization is the billing account in that flow. Treat MCP as a separate integration where the AI host can run the task.

Provider documentation checked on 2026-11-24 10:09 ICT (UTC+07:00). [Sign in with ChatGPT — OpenAI](https://developers.openai.com/siwc/quickstart) · [Gemini API keys — Google](https://aistudio.google.com/docs/api-key) · [Gemini pricing — Google](https://aistudio.google.com/docs/pricing#gemini-3.1-flash-lite) · [Local API — Ollama](https://docs.ollama.com/api/introduction) · [Claude Platform OAuth — Anthropic](https://platform.claude.com/docs/en/manage-claude/oauth-apps/overview)

- [x] Add the connection chooser as the first screen and route initial users to ChatGPT sign-in or the existing demo.
- [x] Add Gemini API key support with server-memory credentials and key validation through model metadata.
- [ ] Run one synthetic Gemini review batch with a real Free tier key and verify usage in the issuing project.
- [ ] Add local Ollama support and verify that the selected downloaded model can process the shared fixture.
- [ ] Verify that each connection reports where usage is counted, shows only its available models and tasks, and can be cleared before switching methods.

Local verification: the Google metadata endpoint rejected a synthetic invalid key and the app kept the session disconnected. Successful inference and billing attribution remain pending a live test.

## Prototype stack

- Use Rust for the backend. Serve a single HTML page from the same process for the local prototype.
- Use [Axum](https://docs.rs/axum/latest/axum/) for the page, OAuth callback, analysis, and disconnect routes. Use [Reqwest](https://docs.rs/reqwest/latest/reqwest/) for HTTPS requests to the selected provider.
- Keep credentials in server-side memory, scoped to the local test session. A restart clears the connection.

## Task — Build and validate a reusable LLM starter template

Give a newcomer a task they can run, understand, and adapt. Use customer-review triage as the first example of turning model output into validated, reusable data. Meeting notes are one possible adaptation. The [workflow simulation](experiments/04-llm-template-use-cases.md) defines the synthetic input and expected review counts.

### Define the template

- [x] Simulate candidate workflows and choose customer review triage as the first example.
- [x] Save a synthetic review fixture and expected output. Include R3 with two issues and R4 with ambiguous text.
- [x] Define a task configuration with input fields, categories, output schema, and validation rules. Document where each field is changed.
- [x] Define the result as editable rows containing a category, source review ID, and exact excerpt, plus a list of unclassified reviews.

### Folder structure for multiple languages

Keep the existing Rust web app at the repository root. Put portable task definitions and fixtures under `templates/`, runnable examples under `starters/`, and experiment notes under `experiments/`.

```text
Cargo.toml
src/                                   # Existing Rust web app and ChatGPT sign-in
templates/
  customer-review-triage/
    task.json                          # Categories, schemas, and validation rules
    sample-input.json
    expected-output.json
    validation-cases.json
starters/
  rust/                                # CLI and validation that the Rust web app can reuse
  python/                              # Beginner-friendly CLI example
  typescript/                          # CLI example for JavaScript and TypeScript developers
experiments/                           # Results and verification notes
```

Each starter should have its own quickstart, entry point, and dependency declarations when needed. For the first task, all starters share the same task definition, sample input, expected output, and rejection cases. A local fixture run passes the manually prepared result through validation and checks counts without requiring a provider account. Compare results independent of row order. Every language must enforce the same category, review ID, exact excerpt, duplicate, and coverage rules.

Use the Rust starter's validation code in the existing web app when wiring in the review task. Keep live provider authorization in the web app for this milestone. Start with Rust and Python, then add TypeScript against the same fixtures. A new task gets a folder under `templates/` once it has its own input, result shape, and expected example.

- [x] Build the Rust starter and expose its validation for reuse.
- [x] Reuse the Rust validator when the review task is wired into the web app.
- [x] Build the Python starter with a quickstart for the same fixture.
- [x] Add the TypeScript starter and verify that all three produce the same result on the shared fixture.
- [x] Add shared negative fixtures for invalid IDs, duplicate classifications, and excerpts missing from source reviews.

### Implement and verify locally

- [x] Add the review task to the Rust prototype using its existing ChatGPT connection and progress display.
- [x] Validate that each category is allowed, each review ID exists, and each excerpt occurs in its source review. Flag invalid rows for review.
- [x] Count unique review IDs per category in Rust. Show the evidence alongside the counts and let the user review or correct rows before CSV export.
- [x] Verify that the fixture returns two late-delivery reviews and two damaged-packaging reviews. Confirm R3 counts once in each category and R4 stays unclassified.
- [x] Exercise empty input, unknown IDs, repeated rows, unsupported excerpts, and ambiguous text. Check that the CSV reflects the reviewed rows and matches the final counts.
- [x] Write a quickstart that a beginner can follow to run the fixture, compare actual and expected output, and change the task configuration.

### Verify that newcomers can try and adapt the examples

- [x] With an eligible test account, run the fixture through ChatGPT and record the returned output, model, time, and redacted evidence in the experiment record.
- [ ] Ask someone unfamiliar with the project to follow the README, run a demo or starter, and identify what was verified, what requires an account, and what remains unverified. Record any setup or documentation gaps.
- [ ] Ask a developer to adapt a starter to a task of their choice. Verify that they can find the task schema, fixtures, and validation rules and reuse the existing provider integration where applicable.
- [ ] Record what newcomers could run and reuse, then use those findings to improve the quickstart or choose the next example.

## Checklist

### Research

- [x] Draft an initial comparison and collect first-party links with publisher and date checked. See [README.md](README.md).
- [ ] Verify which ChatGPT account types and hosted apps can use ChatGPT plan usage, including access and approval requirements.
- [ ] Verify which Claude Platform accounts can link billing and what differs for Claude chat subscriptions and Claude MCP connectors.
- [ ] Verify OpenRouter OAuth consent, who pays, key limits, and how users can revoke access.
- [ ] Compare BYOK and MCP against the goal of running inference and showing results inside the web app.
- [ ] Record each finding with its evidence status, source title, publisher, URL, and date and time checked with timezone.
- [ ] Select the first provider and record the rationale and any unverified requirements.

### Direct provider prototype

The [OpenRouter experiment record](experiments/01-openrouter-oauth.md) contains the build and smoke-test evidence.

- [x] Build the text input, **Analyze with AI** action, connection status, and results view.
- [x] Implement the provider's supported sign-in and consent flow and return the user to the analysis view.
- [x] Associate provider credentials with the correct user on the backend and implement disconnect.
- [x] Run inference with the connected user's credential and render the response in the app.
- [x] Handle declined consent, expired or revoked access, and insufficient quota or credits with actionable messages.

### Verification and publication

- [x] Verify that sign-in and consent complete with the selected provider account.
- [x] Verify that an analysis of synthetic text returns a result inside the web app.
- [ ] Check the provider's usage dashboard or billing records to confirm whose account was charged. Record any attribution that could not be verified.
- [ ] Verify disconnect prevents further requests through the old connection. Test provider-side revocation if supported.
- [ ] Record the provider, account type, model, test date and timezone, result, and redacted evidence for each outcome. Keep documented claims separate from live observations in the [experiment records](README.md#experiments).
- [ ] Document how to run the prototype and publish the evidence with a stable commit link for citation.
