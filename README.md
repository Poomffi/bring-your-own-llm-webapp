# Bring Your Own LLM Web App

This is a research project and a collection of runnable examples for developers and anyone exploring what an LLM app can do. Connect your own AI account or API key, try the account-free review demo, or reuse a starter to build your own integration.

The first provider experiment tests direct authorization, inference in the web app, and whether usage counts against the connected plan or organization. Customer-review triage is one worked example of structured LLM output with validation.

## Current status

| Area | Progress |
| --- | --- |
| ChatGPT | Live sign-in, model discovery, and synthetic review triage were observed. Usage attribution and successful live web search are pending. |
| Gemini | API-key connection, text summaries, and review triage are implemented. Google rejected a synthetic invalid key as expected. Inference and usage with a real key are pending. |
| Review demo | The prepared fixture can be edited, validated, and exported to CSV without an account. |
| Starters | Rust, Python, and TypeScript pass the same reference and nine shared rejection cases. |
| Ollama | Planned. |

See the [web app guide](src/README.md), [starter index](starters/README.md), [template index](templates/README.md), [experiment records](experiments/README.md), and [verification checklist](plan.md).

## Direct provider milestone

1. On the connection chooser, select **Continue with ChatGPT**.
2. Sign in with ChatGPT and approve access.
3. Return to the app, select a task and model, enter sample text, and run the analysis.
4. Check the provider's usage or billing records to confirm whose account covered the request.
5. Disconnect the account and verify that the app can no longer use the connection.

The initial milestone used ChatGPT plan usage. The app shows where text goes and where usage is counted. The server keeps credentials in memory for the active session and clears them on disconnect. Use synthetic text for verification.

## Run the local web app

From the repository root, run `cargo run --locked` with a stable Rust toolchain and open [the local app](http://127.0.0.1:3000). Use that exact loopback address for browser sign-in. On Windows, use the linker required by your Rust toolchain.

The first screen offers **Continue with ChatGPT** and **Connect Gemini** for live inference and **Open review demo** for a prepared sample you can edit without an account. Local Ollama support is planned.

1. Open ChatGPT **Settings → Usage** in the test account and note the app usage shown before the test.
2. Select **Continue with ChatGPT** in the local app. Use a ChatGPT Plus or Pro account that is eligible for plan usage and approve the inference permission.
3. Select an available model and use the sample text. Select **Analyze with AI** once. The app shows the returned model and response ID alongside the result.
4. Open [ChatGPT Settings → Usage](https://chatgpt.com/settings/usage) in the same account and look for usage for the app. If the evidence cannot be tied to the test request, record the attribution as unverified.
5. Select **Disconnect** and confirm the app requires reconnection. Manage provider-side app access in ChatGPT settings.

This local prototype keeps session credentials in server memory and asks the user to reconnect when access expires or the process restarts. The ignored `.chatgpt-host-id` file contains only the installation identifier required for registration.

The app uses the published loopback authorization flow and requests separate permission for ChatGPT plan usage. It lists models with the authorized token and calls the public Responses API. [Registration and sign-in — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/sign-in) · [Models and inference — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)

## Run with a Gemini API key

1. Open the local app at the loopback address above. Paste a Gemini Developer API key from Google AI Studio into **Gemini API key** and select **Connect Gemini**. The app checks the key and retrieves model metadata from Google.
2. Select **Summarize text** or **Triage customer reviews** with **Gemini 3.1 Flash-Lite**. For review triage, paste the synthetic review texts from [the sample input](templates/customer-review-triage/sample-input.json), one per line.
3. Run the task once and check usage in Google AI Studio. Google receives the submitted text. Requests draw from the quota or billing of the Google project that issued the key. Check the project's tier before testing.
4. Select **Disconnect and change** to clear the key from the local server session. The credential also expires after one hour or when the server stops. Manage or revoke the key in Google AI Studio.

Google's model metadata and generation endpoints were checked on 2026-11-24 10:31 ICT (UTC+07:00). [Get model — Google](https://aistudio.google.com/api-reference/models/get-model) · [Generate content — Google](https://aistudio.google.com/api-reference/generate-content/generate-content) · [Gemini 3.1 Flash-Lite pricing — Google](https://aistudio.google.com/docs/pricing#gemini-3.1-flash-lite)

## Triage customer reviews

1. Select **Task → Triage customer reviews → Preview sample result** to open the four synthetic reviews and their reference classifications.
2. Inspect the source review IDs and quotes. Edit, add, or remove rows, or mark source reviews as unclassified. Choose **Check rows** to verify the edits. The Rust validator highlights the first invalid row when an issue can be tied to a row.
3. Select **Download CSV** after the rows pass validation. The CSV contains classified rows and their exact excerpts. Unclassified reviews remain visible in the review panel.
4. For new classifications, connect ChatGPT or Gemini, enter one review per line, choose a model, and select **Classify reviews with AI**. Compare the classifications with the source reviews before export.

The [template experiment](experiments/04-llm-template-use-cases.md) records local results. Live classification and usage attribution are tracked separately for each provider.

## Starter repository layout

```text
src/                                  Rust web app and provider integrations
templates/
  customer-review-triage/
    task.json                         Categories, schemas, and validation rules
    sample-input.json                 Synthetic customer reviews
    expected-output.json              Expected rows and unique counts
    validation-cases.json              Shared examples of rejected results
starters/
  rust/                               CLI and reusable Rust validator
  python/                             Standard-library CLI
  typescript/                         Node.js CLI
experiments/                          Research and test evidence
```

From the repository root, pick a language and run its fixture check.

| Language | Command | Guide |
| --- | --- | --- |
| Rust | `cargo run --locked --manifest-path starters/rust/Cargo.toml` | [Rust](starters/rust/README.md) |
| Python | `python starters/python/main.py` | [Python](starters/python/README.md) |
| TypeScript | `node starters/typescript/main.ts` | [TypeScript](starters/typescript/README.md) |

The starters read the same files and verify the reference result and rejection cases locally. Pick a starter in your language, then change the categories in `task.json` and add a corresponding input and expected result. The first validator covers the customer-review row shape. A different shape, such as meeting action items, needs its own validation logic. See the [template experiment](experiments/04-llm-template-use-cases.md) for the design.

## Experiments

Each file records how to reproduce the experiment, what was observed, and what remains to verify.

| Experiment | Status |
| --- | --- |
| [OpenRouter OAuth](experiments/01-openrouter-oauth.md) | Local OAuth checks passed |
| [Direct ChatGPT text analysis](experiments/02-chatgpt-text.md) | Local checks passed. A [later review run](experiments/04-llm-template-use-cases.md#live-chatgpt-review-test) verified live inference |
| [Direct ChatGPT web search](experiments/03-chatgpt-web-search.md) | Build and fixture checks passed. Live search pending |
| [LLM starter template use cases](experiments/04-llm-template-use-cases.md) | Shared fixtures passed in three languages. Live ChatGPT review triage succeeded |

OpenAI lists image generation as unsupported for this ChatGPT plan-usage integration. [Plan usage preview limitations — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations)

## Initial provider research

Official documentation checked on 2026-11-21 at 13:59 ICT (UTC+07:00). See the experiment records for later live observations and the remaining billing and disconnect checks.

| Candidate | Documented behavior | Still to verify for this project |
| --- | --- | --- |
| OpenRouter OAuth | OpenRouter documents a PKCE flow that exchanges an authorization code for a user-controlled API key. [OAuth PKCE — OpenRouter](https://openrouter.ai/docs/guides/overview/auth/oauth) | Test account eligibility, usage attribution, key limits, and revocation. |
| ChatGPT plan usage | OpenAI documents plan usage for eligible users and a waitlist for paid or remotely hosted apps. [Integrating Sign in with ChatGPT in your open-source app — OpenAI](https://developers.openai.com/cookbook/articles/sign-in-with-chatgpt) | Whether the intended hosted app and test account qualify, and what approval they require. |
| Claude Platform Linked Billing | Anthropic documents a beta flow that can bill API usage to an eligible Claude organization after a user with the Admin or Billing role authorizes the app. [Claude Platform OAuth — Anthropic](https://platform.claude.com/docs/en/manage-claude/oauth-apps/overview) | Test organization eligibility and app testing or review requirements. |

Gemini is the first bring-your-own-key (BYOK) integration. Inference with a real key remains to be verified. The research will also compare an MCP integration where the analysis runs in the AI host.

See [plan.md](plan.md) for the research, implementation, and verification checklist and [AGENTS.md](AGENTS.md) for contributor instructions.
