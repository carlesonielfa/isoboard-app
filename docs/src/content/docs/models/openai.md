---
title: Use isoboard with an OpenAI API key
description: Connect your own OpenAI API key to isoboard to convert screenshots with OpenAI models. Learn where to enter the key, how it is stored and how billing works.
sidebar:
  label: OpenAI
  order: 3
---

<!-- os:macos -->
You can use isoboard with your own OpenAI API key. Create a key in your OpenAI account, paste it into the **OpenAI** row on the **Providers** tab, and pick an image-capable model in a Profile. isoboard stores the key in the macOS Keychain, and OpenAI bills you directly for usage.

## How do I connect OpenAI?

1. Sign in to the [OpenAI API keys page](https://platform.openai.com/api-keys) and create a new secret key. Copy it when it is shown.
2. In isoboard, open Settings and select the **Providers** tab.
3. Expand **OpenAI** and paste the key into **API key**.
4. Click outside the field. A status below it shows **Ready** or **Configured**. Configured means the key is saved and isoboard has not used it yet.

isoboard then creates two starting Profiles, **OpenAI API - Efficient** and **OpenAI API - Accurate**. Efficient favors faster, lower-cost captures. Accurate favors difficult captures and may take longer and cost more. You can edit or delete both. See [Profiles](../../use/profiles/).

:::tip[Already pay for ChatGPT Pro or Business?]
Some ChatGPT plans include monthly API credits that cover captures in isoboard. ChatGPT Plus does not. See [use your ChatGPT or Claude plan with isoboard](../subscription-api-credits/).
:::

## Which OpenAI models work?

Any OpenAI model that accepts images as input. In the **Profiles** tab, choose **OpenAI** as the **Provider**, then search the model picker by name or type a model ID. Models that do not accept images fail at capture time. isoboard does not block them when you save.

## Where does isoboard store my key?

<!-- os:macos -->
In the macOS Keychain, never in the configuration file. isoboard does not show the key again after you save it, and you replace it by pasting a new one. See [where isoboard stores your data](../../privacy/data/).

## Who pays for the usage?

You do. Each capture sends an image to OpenAI, and OpenAI charges your account under its own pricing. isoboard does not add a fee. The isoboard license is separate. See [pricing and licensing](../../license/pricing-and-licensing/).

## What do OpenAI errors mean?

| Message in isoboard | Likely cause |
|---|---|
| the API key is invalid, or provider authentication is required | The key was revoked, mistyped or belongs to another account. Paste a new key. |
| provider access was denied | Your account or project cannot use that model. |
| provider inference is rate limited | You hit an OpenAI rate limit or ran out of credit. Wait, or check billing. |
| inference timed out | The request took longer than the Profile's timeout. |

More fixes: [provider errors](../../troubleshooting/provider-errors/).

## Related

- [Choose a local model or an AI provider](../choose-a-model/)
- [Profiles](../../use/profiles/)
- [What leaves your Mac](../../privacy/data/)
