# Direct ChatGPT text analysis

This experiment checks whether a signed-in user's ChatGPT plan covers a text analysis displayed in the Rust app. The app requests permission for inference and uses the public Responses API. [Registration and sign-in — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/sign-in) · [Models and inference — OpenAI](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference)

## How to run

1. Open the [local app](http://127.0.0.1:3000) and select **Continue with ChatGPT**. Authorize plan usage with an eligible account.
2. Choose **Task → Summarize text** and an available model. Use the synthetic sample and select **Analyze with AI** once.
3. Record the returned model, response ID, and test time with timezone. Check for the app's usage in [ChatGPT Settings → Usage](https://chatgpt.com/settings/usage).
4. Disconnect and verify that the app requires a new connection before it can analyze again.

## Observed locally

On 2026-11-21 at 14:48 ICT (UTC+07:00), the locked Windows build and two Rust tests passed. HTTP checks covered the direct authorization URL, loopback callback, one-time state, and inference gating. The browser also reached OpenAI's account picker. A later UI simulation verified the progress display with fixture events.

## Live verification

Consent, model discovery, inference, usage attribution, and disconnect after successful sign-in remain pending. Record each outcome separately. If the usage visible in ChatGPT cannot be tied to this run, keep attribution open.
