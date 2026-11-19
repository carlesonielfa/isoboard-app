---
title: Use isoboard with an Anthropic API key
description: Connect your own Anthropic API key to isoboard to convert screenshots with Claude models. Learn where to enter the key, how it is stored and how billing works.
sidebar:
  label: Anthropic
  order: 4
---

<!-- os:macos -->
You can use isoboard with your own Anthropic API key. Create a key in the Anthropic Console, paste it into the **Anthropic** row on the **Providers** tab, and pick a Claude model in a Profile. isoboard stores the key in the macOS Keychain, and Anthropic bills you directly.

## How do I connect Anthropic?

1. Sign in to the [Anthropic Console API keys page](https://platform.claude.com/settings/keys) and create a key. Copy it when it is shown.
   2. In isoboard, open Settings and select the **Providers** tab.
3. Expand **Anthropic** and paste the key into **API key**.
4. Click outside the field. A status below it shows **Ready** or **Configured**. Configured means the key is saved and isoboard has not used it yet.

isoboard then creates two starting Profiles, **Anthropic - Efficient** and **Anthropic - Accurate**. Efficient favors faster, lower-cost captures. Accurate favors difficult captures and may take longer and cost more. You can edit or delete both. See [Profiles](../../use/profiles/).

:::tip[Already pay for Claude Max, Ultra or Team?]
Some Claude plans include monthly API credits that cover captures in isoboard. Claude Pro does not. See [use your ChatGPT or Claude plan with isoboard](../subscription-api-credits/).
:::

## Which Claude models work?

Any Claude model that accepts images. In the **Profiles** tab, choose **Anthropic** as the **Provider**, then search the model picker by name or type a model ID. A model without image input fails at capture time. isoboard does not block it when you save.

## Where does isoboard store my key?

<!-- os:macos -->
In the macOS Keychain, never in the configuration file. isoboard does not show the key again after you save it, and you replace it by pasting a new one. See [where isoboard stores your data](../../privacy/data/).

## Who pays for the usage?

You do. Each capture sends an image to Anthropic, and Anthropic charges your account under its own pricing. isoboard adds no fee. The isoboard license is separate. See [pricing and licensing](../../license/pricing-and-licensing/).

## What do Anthropic errors mean?

| Message in isoboard | Likely cause |
|---|---|
| the API key is invalid, or provider authentication is required | The key was deleted, mistyped or belongs to another workspace. Paste a new key. |
| provider access was denied | Your account cannot use that model. |
| provider inference is rate limited | You hit a rate limit or your credit balance is empty. |
| inference timed out | The request took longer than the Profile's timeout. |

More fixes: [provider errors](../../troubleshooting/provider-errors/).

## Related

- [Choose a local model or an AI provider](../choose-a-model/)
- [Profiles](../../use/profiles/)
- [What leaves your Mac](../../privacy/data/)
