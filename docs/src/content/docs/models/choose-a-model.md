---
title: Choose a local model or an AI provider
description: Compare isoboard's local isOCR model with OpenAI, Anthropic, Gemini and OpenAI-compatible providers on privacy, cost, speed, quality and offline use.
sidebar:
  label: Choose a model
  order: 1
---

isoboard converts captures with a local model that runs on your Mac, or with an AI provider you connect using your own API key. The local model keeps images on your device, works offline after a one-time download and has no usage fees. A provider is the option for demanding captures and Additional Rules.

## Local model or AI provider: how do they compare?

| | Local model (isOCR™) | AI provider (OpenAI, Anthropic, Gemini) | OpenAI-compatible server |
|---|---|---|---|
| Where the image goes | Stays on your Mac | Sent to the provider | Sent to your endpoint |
| Account or key | None | Your own API key | An endpoint URL, plus a key if the server needs one |
| Cost | No usage fees | The provider bills you per use | Depends on the server |
| Offline | Yes, after the model is downloaded | No | Only if the server runs on your Mac or network |
| Setup | One download of about 1 GB | Paste a key | Run a server, enter its URL and a model ID |
| Additional Rules | Needs a separate text-capable model | Supported | Supported if the model accepts text |

Cost, speed and quality of a provider depend on the model you pick and on your account. Some ChatGPT and Claude plans include monthly API credits that cover isoboard captures. See [use your ChatGPT or Claude plan credits](../subscription-api-credits/).

On easy to medium captures, expect good results from isOCR. It preserves more styling than general-purpose models and usually runs much faster. Difficult captures are where a larger provider model can do better.

## Which should I use first?

Start with isOCR. It is the model the setup guide recommends for day-to-day use, it needs no account, and your first capture works as soon as the download finishes.

Add a provider later if you need one of these:

- Captures that isOCR gets wrong, such as dense or unusual layouts.
- [Additional Rules](../../use/additional-rules/), for example translation. isOCR only extracts text, so a rules step needs a text-capable model.

The current catalog entry is isOCR1 0.8B. Check its output on your own screenshots before relying on it.

You can keep both. Each [Profile](../../use/profiles/) chooses its own model, so one Profile can use isOCR and another a provider.

## What about privacy?

With a local model, the conversion happens on your Mac. With a provider, isoboard sends the image to that provider. If you convert locally and use a provider only for Additional Rules, only the extracted content goes to the provider. Choose local models for both steps to keep the whole workflow on your Mac.

A provider applies its own terms and retention rules to what you send. See [what leaves your Mac](../../privacy/data/) for the full picture.

## How do I set up each option?

- Local model: [Run isOCR and other local models](../local-models/)
- [OpenAI](../openai/), [Anthropic](../anthropic/) or [Google Gemini](../gemini/) with your own key
- [Ollama, LM Studio and other OpenAI-compatible servers](../openai-compatible/)

## Frequently asked questions

**Do I need an API key to use isoboard?**

No. isOCR runs on your Mac without an account or API key. You need a key only to use OpenAI, Anthropic, Gemini or an OpenAI-compatible server, and those providers bill you directly.

**Can I use isoboard offline?**

Yes, once the local model is downloaded. The first download needs an internet connection. Providers need a connection to their service, unless an OpenAI-compatible server runs on your own Mac or network.

**Does isoboard pay for my provider usage?**

No. Provider charges are between you and the provider. The isoboard license is separate. See [pricing and licensing](../../license/pricing-and-licensing/).

## Related

- [Run isOCR and other local models](../local-models/)
- [Profiles](../../use/profiles/)
- [What leaves your Mac](../../privacy/data/)
