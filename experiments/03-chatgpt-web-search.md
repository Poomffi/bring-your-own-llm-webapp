# Direct ChatGPT web search

This experiment checks whether an existing ChatGPT plan can run a web search from the Rust app and return a short answer with clickable sources. OpenAI documents web search as dependent on model and account or workspace policy. Documentation checked on 2026-11-21 at 16:18 ICT (UTC+07:00). [Plan usage preview limitations — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations) · [Web search — OpenAI](https://developers.openai.com/api/docs/guides/tools-web-search)

## How to run

1. Connect an eligible ChatGPT account in the [local app](http://127.0.0.1:3000).
2. Select **Task → Search the web**, choose a model, and send the sample question about Artemis II.
3. Look for the search progress, answer, and links. Record the model, response ID, time with timezone, and a redacted example of the returned citations.
4. Check [ChatGPT Settings → Usage](https://chatgpt.com/settings/usage) for usage associated with the connected app.

## Observed locally

On 2026-11-21, the locked Windows build and all five Rust tests passed. Fixture tests verified that search mode requires the web search tool, updates progress from search events, and renders clickable citations. Local HTTP checks confirmed both modes are present and that signed-out search cannot run.

## Live verification

On 2026-11-23 ICT (UTC+07:00), the browser showed an active ChatGPT plan connection and a model catalog. A search with GPT-6.2-Sol ended with the app's generic incomplete-analysis error. The original server discarded the provider error, so the cause is unverified.

The revised app distinguishes provider errors, incomplete results, and interrupted streams. It records the request ID, stage, and a machine error code when available. A completed live search, citations, and usage attribution remain pending. [Errors and recovery — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery) · [Models and inference — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)

A second live attempt on 2026-11-23 reached OpenAI's completion event, but the app could not extract an answer from the final response object. The logs for that attempt did not record text fragments, so whether OpenAI sent any earlier remains unknown. The parser now assembles streamed text by output and content index, retains URL annotations, and uses that text when the completion object lacks an answer. Two local regression tests passed. A live search with the revised parser is pending.
