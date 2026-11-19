---
title: isoboard frequently asked questions
description: Short answers about isoboard, the Mac app that turns screenshots into pasteable text and tables. Platforms, privacy, offline use, providers, cost and history.
sidebar:
  label: FAQ
  order: 4
---

isoboard is a macOS menu bar app that turns a screenshot into rich text, tables and images you can paste into other apps. It uses a local model on your Mac or your own AI provider key. These answers are short, and each links to the page that covers the topic in full.

**Which platforms does isoboard support?**

<!-- os:macos -->
isoboard runs on macOS 14 Sonoma or later. Windows and Linux are coming, prioritized by demand. [Vote for your platform](../../community/feedback/). See [what is isoboard](../../getting-started/what-is-isoboard/) and the [install guide](../../getting-started/install/).

**Do I need an API key?**

Not for the local model. isOCR™ runs on your Mac without an account or usage fees. You need your own key only to use OpenAI, Anthropic, Gemini or an OpenAI-compatible server, and those providers bill you directly. See [choose a local model or an AI provider](../../models/choose-a-model/).

**Does my content leave my device?**

With a local model, conversion happens on your Mac. With an AI provider, isoboard sends the image to that provider. If you convert locally and use a provider only for Additional Rules, only the extracted content is sent. See [what leaves your Mac](../../privacy/data/).

**Can I use isoboard offline?**

Yes, once the local model is downloaded. The first download is about 1 GB and needs an internet connection. A provider needs a connection to its service, unless an OpenAI-compatible server runs on your own Mac or network. See [local models](../../models/local-models/).

**Which AI providers can I use?**

OpenAI, Anthropic and Gemini with your own API key, and any OpenAI-compatible server such as Ollama or LM Studio. Available models depend on the provider and your account. Some ChatGPT and Claude plans include API credits. See the [model guides](../../models/choose-a-model/) and [plan API credits](../../models/subscription-api-credits/).

**Can it translate or follow custom instructions?**

Yes. [Additional Rules](../../use/additional-rules/) can translate or reformat the result with a text-capable model. isOCR only extracts content, so pair it with another model in a separate rules step. Save the combination as a [Profile](../../use/profiles/) to reuse it.

**Can I capture from any app?**

Almost. isoboard needs the Screen Recording permission. Some apps and protected content, such as streaming video, block screen capture and appear blank. See [fix capture and permission problems](../../troubleshooting/capture-and-permissions/) and [grant macOS permissions](../../getting-started/permissions/).

**Where can I paste the result?**

Into apps that accept rich clipboard content. Microsoft Word, Excel and Google Docs keep almost everything, slide apps need some rearranging, and some apps keep only the text. See the [list of tested apps](../../use/paste/#which-apps-support-isoboard-results).

**What does History save?**

History is stored on your Mac. It keeps results, a small thumbnail of each source and a summary of the settings used. It also keeps the original screenshot of your 50 most recent captures unless you turn off **Keep original screenshots** in Settings > General. You can copy a past result again or delete entries. See [History](../../use/history/).

**Does isoboard collect telemetry?**

No. isoboard sends no telemetry or analytics. See [what leaves your Mac](../../privacy/data/).

**Is isoboard free?**

Personal use is free. Using isoboard for work needs a €50 commercial license after a seven-day evaluation. Your AI provider bills you separately, and the local model has no usage fees. See [pricing and licensing](../../license/pricing-and-licensing/).

**How do I change the capture shortcut?**

Open the **Shortcuts** tab, select the shortcut field and press the new combination. The default is `⌘ Cmd` + `⇧ Shift` + `2`. See [keyboard shortcuts](../keyboard-shortcuts/).

**How do I uninstall isoboard?**

Turn off **Open at login**, quit and delete the app, then delete its data folder and Keychain items. See [uninstall isoboard](../../troubleshooting/uninstall/).

## Related

- [Licensing FAQ](../../license/faq/)
- [Settings reference](../settings/)
- [Release notes](../release-notes/)
