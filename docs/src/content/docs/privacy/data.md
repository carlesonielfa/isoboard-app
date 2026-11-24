---
title: What data does isoboard send or store?
description: See what isoboard sends over the network in each mode, what it stores on your Mac, and how to delete it. Local isOCR keeps captures on your device.
sidebar:
  label: Your data
  order: 1
---

With a local model, isoboard converts captures on your Mac and sends them nowhere. With an AI provider or an OpenAI-compatible server, the capture goes to that service. isoboard has no server of its own that receives your captures. History, settings and API keys stay on your Mac.

:::note
This page summarizes the [privacy notice](https://isoboard.app/legal/privacy/), which governs if anything here differs.
:::

<!-- os:macos -->
Storage paths and Keychain names on this page are for macOS.

## What leaves my Mac in each mode?

| Mode or service | What is sent | Where it goes |
|---|---|---|
| Local model (isOCR™) | Nothing | Stays on your Mac |
| AI provider (OpenAI, Anthropic, Gemini) | The image, extracted text and your instructions, depending on the step | Directly to the provider, with your own API key |
| OpenAI-compatible server | The same content | The endpoint you configured. It can be on your Mac or network |
| Additional Rules with a separate provider | The extracted content, not the image | The provider chosen for the rules step |
| License validation | Your license key, product identifiers, app version and connection information such as your IP address | Polar |
| Model download | An ordinary web request for the model files | Hugging Face and its delivery providers |
| Model catalog refresh | An ordinary web request for model metadata, once each time isoboard starts | models.dev |
| Update check | An ordinary web request for the release manifest, and for the update itself when one is available. Turn off automatic checks in **Settings** → **About** | GitHub |

None of these requests passes through an isoboard-operated server. Model downloads, catalog refreshes, update checks and license checks never include your captures. isoboard sends no telemetry or analytics.

### What stays on my Mac when I use a local model?

If you use local models for both conversion and Additional Rules, processing happens on your device. The only network traffic is the one-time model download, catalog refreshes, update checks and license checks. See [run isOCR and other local models](../../models/local-models/).

### What does an AI provider receive?

With a provider, the relevant image, extracted text and instructions go directly to that provider. If you convert locally and use a provider only for Additional Rules, only the extracted content goes to it. Each provider applies its own terms and retention rules. isoboard cannot promise that a provider does not keep inputs or use them for training. Review the provider's terms before sending sensitive material. See [choose a local model or an AI provider](../../models/choose-a-model/).

### What does the license check send to Polar?

isoboard sends your license key, product identifiers, app version and connection information to Polar to validate your license. It sends no captures, device fingerprints, computer names or usage counts. Evaluation and personal use make no license requests. See [buy and activate a license](../../license/activate/).

### What do Hugging Face, models.dev and GitHub receive?

Downloading isOCR connects to Hugging Face and its delivery providers. Refreshing the model catalog connects to models.dev. Checking for and downloading updates connects to GitHub. Each sees ordinary connection information such as your IP address and the requested resource. None of these requests includes your captures.

## What does isoboard store on my Mac?

<!-- os:macos -->
On macOS, the app keeps its files in `~/Library/Application Support/app.isoboard/`.

| What | Where | Contents |
|---|---|---|
| Settings and Profiles | `config.toml` in the folder above | Shortcuts, Profiles and provider settings. No API keys. See [the configuration file](../../reference/configuration-file/) |
| History | `history.sqlite3` in the same folder | Extracted content, results, a small source thumbnail, and processing information |
| Original screenshots | `history-originals/` in the same folder | The full-resolution image of your 50 most recent captures, unless you turn off **Keep original screenshots** |
| Downloaded models | `models/` in the same folder | isOCR and other local model files |
| Evaluation dates | `licensing.json` in the same folder | When your seven-day evaluation started |
| License status | `commercial-license.json` in the same folder | License status. Not the full key or billing data |
| Model catalog cache | `models.dev.catalog.json` in the macOS cache folder `~/Library/Caches/app.isoboard/` | Public model metadata that isoboard can re-download |

<!-- os:macos: Windows uses Credential Manager -->
API keys and your license key are stored in the macOS Keychain, in items whose names start with `app.isoboard`. They are not written to `config.toml`.

### What does History keep?

History keeps a fixed number of recent captures and has no time-based expiry. See [find past captures in History](../../use/history/) for the limits. Entries hold the extracted content, results, a small thumbnail and a record of the Profile and provider settings used, without credentials, endpoints or rule text. By default isoboard also keeps the full-resolution screenshot of your 50 most recent captures, up to 1 GB, so they can be processed again. Older entries keep only their thumbnail. Turn off **Keep original screenshots** in Settings > General to stop this and delete the screenshots already kept.

### Does isoboard read my other files?

Installing isoboard does not give its author access to your local files. isoboard processes only the screen regions you capture.

## How do I delete my data?

| To delete | Do this |
|---|---|
| One History entry | Open the **History** tab, open the entry's menu and choose **Delete** |
| All History | Quit isoboard and delete `history.sqlite3`. The History tab has no clear-all action |
| A downloaded model | Use the remove control next to the model. See [local models](../../models/local-models/) |
| An API key | **Providers** tab → expand the provider → click the trash button next to **API key** and confirm. See the key section of the provider page ([OpenAI](../../models/openai/), [Anthropic](../../models/anthropic/), [Gemini](../../models/gemini/) or [OpenAI-compatible](../../models/openai-compatible/)) and the [Keychain steps](../../troubleshooting/uninstall/#remove-api-keys-and-the-license-key-from-the-keychain) |
| The license key | **Settings** tab → **License** → **Remove license**. See [buy and activate a license](../../license/activate/) |
| Settings and Profiles | See [reset the configuration](../../troubleshooting/reset-configuration/) |
| Everything | See [uninstall isoboard](../../troubleshooting/uninstall/) |

<!-- os:macos -->
To delete Keychain items by hand, open Keychain Access, search for `app.isoboard`, and delete the matching items.

Deleting an entry does not remove copies in your clipboard history, backups, clipboard managers or the apps where you pasted. Deleting `history.sqlite3` or a History entry is not a guarantee of secure erasure. Uninstalling the app can leave this folder behind, so remove it for a complete clean-up.

Removing the license key does not cancel your purchase or delete records that Polar holds. Contact legal@isoboard.app for privacy requests. The [privacy notice](https://isoboard.app/legal/privacy/) describes your rights.

## Frequently asked questions

**Does isoboard send my screenshots to the cloud?**

Only if you choose a cloud provider or a server that is not on your Mac. With isOCR, conversion happens on your device. The image goes to a provider or OpenAI-compatible endpoint only when you select one for conversion.

**Does isoboard keep my screenshots?**

No. History stores extracted content, results and a small thumbnail. It does not retain the original full screenshot.

**Does isoboard collect telemetry or analytics?**

No. isoboard sends no telemetry or analytics. The only network requests are the ones in the table above, and none carries your captures. The [privacy notice](https://isoboard.app/legal/privacy/) covers the legal terms.

## Related

- [Choose a local model or an AI provider](../../models/choose-a-model/)
- [Find past captures in History](../../use/history/)
- [Pricing and licensing](../../license/pricing-and-licensing/)
- [Privacy notice](https://isoboard.app/legal/privacy/)
