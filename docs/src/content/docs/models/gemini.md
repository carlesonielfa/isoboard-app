---
title: Use isoboard with a Google Gemini API key
description: Connect your own Google Gemini API key to isoboard to convert screenshots with Gemini models. See where to enter the key, how it is stored and billed.
sidebar:
  label: Google Gemini
  order: 6
---

<!-- os:macos -->
You can use isoboard with your own Google Gemini API key. Create a key in Google AI Studio, paste it into the **Gemini** row on the **Providers** tab, and pick a Gemini model in a Profile. isoboard stores the key in the macOS Keychain, and Google bills you directly where your plan charges for use.

## How do I connect Gemini?

1. Open [Google AI Studio's API key page](https://aistudio.google.com/apikey) and create a key. Copy it.
2. In isoboard, open Settings and select the **Providers** tab.
3. Expand **Gemini** and paste the key into **API key**.
4. Click outside the field. The key is saved and the field shows dots (••••••••) in its place.

isoboard then creates two starting Profiles, **Gemini - Efficient** and **Gemini - Accurate**. Efficient favors faster, lower-cost captures. Accurate favors difficult captures and may take longer and cost more. You can edit or delete both. See [Profiles](../../use/profiles/).

## Which Gemini models work?

Any Gemini model that accepts images. In the **Profiles** tab, choose **Gemini** as the **Provider**, then search the model picker by name or type a model ID. A model without image input fails at capture time. isoboard does not block it when you save.

## Where does isoboard store my key?

<!-- os:macos -->
In the macOS Keychain, never in the configuration file. isoboard does not show the key again after you save it. A saved key appears as dots in **API key**. Paste a new key to replace it, or click the trash button next to the field and confirm **Remove key** to delete it from this Mac. See [where isoboard stores your data](../../privacy/data/).

## Who pays for the usage?

You do. Each capture sends an image to Google, and Google applies its own pricing, free-tier limits and terms to your key. isoboard adds no fee. The isoboard license is separate. See [pricing and licensing](../../license/pricing-and-licensing/).

As of November 2026, the Gemini API has a free tier for some models. On the free tier, Google may use your prompts and responses to improve its products, and human reviewers may read them. Paid usage is not used that way. Do not send sensitive or confidential captures on a free-tier key. Check the current terms on [Google's Gemini API pricing page](https://ai.google.dev/gemini-api/docs/pricing) before you rely on this.

## What do Gemini errors mean?

| Message in isoboard | Likely cause |
|---|---|
| the API key is invalid, or provider authentication is required | The key was deleted, mistyped or restricted. Paste a new key. |
| provider access was denied | Your project or region cannot use that model. |
| provider inference is rate limited | You hit a quota. Wait, or check your plan. |
| inference timed out | The request took longer than the Profile's timeout. |

More fixes: [provider errors](../../troubleshooting/provider-errors/).

## Related

- [Choose a local model or an AI provider](../choose-a-model/)
- [Profiles](../../use/profiles/)
- [What leaves your Mac](../../privacy/data/)
