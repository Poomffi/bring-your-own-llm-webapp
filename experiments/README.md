# Experiment records

These notes track local tests, live provider observations, and open checks.

| Record | Latest evidence |
| --- | --- |
| [OpenRouter OAuth](01-openrouter-oauth.md) | Local OAuth checks passed. Live consent and inference were left open when the prototype moved to ChatGPT. |
| [ChatGPT text analysis](02-chatgpt-text.md) | Authorization and analysis gating were checked locally. The later review experiment records a successful live request. |
| [ChatGPT web search](03-chatgpt-web-search.md) | The revised parser passed local tests after live response handling errors. A successful live run is pending. |
| [Reusable review starter](04-llm-template-use-cases.md) | Shared fixtures passed in three languages. Live ChatGPT review triage matched the expected counts and the CSV was ready to export. |

Gemini key validation reached Google and rejected a synthetic invalid key. Real-key inference and usage attribution are open in the [plan](../plan.md).
