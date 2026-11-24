# AGENTS.md

## Project

Build a web app where users can connect their own LLM account and use it to analyze text. The first prototype starts with a provider sign-in and consent flow, then displays the analysis in the app. The user's connected account should cover inference charges. Study bring-your-own-key (BYOK) as a fallback. See [plan.md](plan.md) for the current scope and [README.md](README.md) for the initial research and sources.

## Research

- Prefer official provider documentation, product terms, and API references. For each material claim, record the source title, publisher, direct URL, date and time checked with timezone, and the specific claim it supports.
- Distinguish app sign-in, provider authorization for inference, and billing. Record the eligible account or plan, provider approval requirements, who pays, and where results appear.
- Label conclusions as documented, observed in a live test, or still unverified. Record successful consent, inference, billing attribution, and disconnect as separate outcomes.
- Recheck provider-specific limits and eligibility before building against them. Use published integration paths and report any approval or account requirement that blocks a test.

## Implementation

- Keep the first version to one text-analysis feature and one provider. Choose the provider from the research and use the simplest supported integration.
- Keep provider credentials server-side, scoped to the appropriate user session or account. Follow the provider's authorization requirements and keep tokens, keys, and prompts out of logs and the repo.
- Explain where the input will be sent and who pays before the first analysis. Provide a way to disconnect and clear credentials stored by the app. Document provider-side revocation separately if it requires a provider action.
- Use synthetic text for example requests and verification. Update the checklist in `plan.md` as evidence is collected.
