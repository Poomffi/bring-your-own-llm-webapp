# OpenRouter OAuth

The experiment checked whether the Rust app could start OpenRouter OAuth and receive a loopback callback. Paid requests through this path would draw from the connected OpenRouter account or workspace credits. [OAuth PKCE — OpenRouter](https://openrouter.ai/docs/guides/overview/auth/oauth) · [Support — OpenRouter](https://openrouter.ai/support/)

## Observed locally

On 2026-11-21 at 14:29 ICT (UTC+07:00), the locked Windows build passed. Local HTTP checks covered the page, the PKCE authorization URL, rejection of a mismatched callback state, and session invalidation on disconnect.

## Live verification

Consent, inference, credit attribution, and disconnect after successful connection were pending when the project moved to ChatGPT plan usage.

## Decision

Keep this result as evidence for the OAuth scaffold. Use the [direct ChatGPT experiment](02-chatgpt-text.md) to test usage under an existing ChatGPT plan.
