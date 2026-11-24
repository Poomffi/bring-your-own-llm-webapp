# Rust web app

Follow the [root quickstart](../README.md#run-the-local-web-app) to start the app and choose a provider.

| File | Role |
| --- | --- |
| [main.rs](main.rs) | Routes, session credentials, provider selection, and progress delivery. |
| [chatgpt.rs](chatgpt.rs) | Direct ChatGPT authorization, model discovery, and Responses API integration. |
| [gemini.rs](gemini.rs) | Gemini API-key validation and text generation. |
| [index.html](index.html) and [review.js](review.js) | Connection chooser, task form, progress, and review editor. |
| [review.rs](review.rs) | Shared fixture, Rust validator integration, and CSV export. |

**Verified** · Live ChatGPT review triage, the local review demo, and rejection of an invalid Gemini key.

**Pending** · Live Gemini inference, successful live web search, usage attribution, and Ollama. See the [plan](../plan.md).
