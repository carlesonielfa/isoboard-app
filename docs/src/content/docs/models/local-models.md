---
title: Run isOCR and other local models on your Mac
description: isOCR is isoboard's on-device OCR model. Learn its download size, how to download, cancel, resume and remove it, where files live, and how to use it offline.
sidebar:
  label: Local models and isOCR
  order: 2
---

isOCR™ is the OCR model built for isoboard. It runs on your Mac, so captures stay on your device and work offline once the model is downloaded. It needs no account or API key. You download about 1 GB once, and conversions need about 2 GB of free memory. You can pause the download and resume it later. For how isOCR compares with cloud models, see [choose a local model or an AI provider](../choose-a-model/).

## What is isOCR?

isOCR is a model fine-tuned for turning screenshots into text, tables and images. The current version is isOCR1 0.8B.

isOCR only extracts content. [Additional Rules](../../use/additional-rules/), such as translation, need a separate text-capable model, which you can choose in the same Profile. isOCR never falls back to a cloud provider on its own.

## How do I download isOCR?

The first-run setup guide offers the download. You can also do it later:

1. Open Settings and select the **Profiles** tab.
2. Pick a Profile and set **Provider** to **Local**.
3. Choose **isOCR1 0.8B** in the model picker.
4. Select the download button (**Download model**).

Before the download starts, isoboard shows the model's license. Downloading the model means you accept it.

If a local model is not downloaded when you capture, isoboard downloads it first and shows the progress in the capture overlay and footer.

## How much memory and disk space does isOCR need?

| Requirement | isOCR1 0.8B |
|---|---|
| Download | About 1 GB |
| Free disk space | About 1 GB |
| Memory | About 1.5 GB at peak during a capture. Plan for 2 GB of free memory. |

<!-- os:macos -->
isoboard shows the exact size on the download button before you start. The model loads into memory when you capture. A capture after a pause shows **Loading model** while the model starts.

## What happens if the download stops?

- **Cancel:** select the cancel button (**Cancel download; keep partial files for resume**). The partial files stay on your Mac.
- **Resume:** select **Resume model download** and the download continues from where it stopped, including after you quit and reopen isoboard.
- **Failure:** select **Retry model download**, or capture again to retry automatically.
- **Verification:** isoboard checks the downloaded files before the model becomes available. You see **Verifying** at the end.

While a download runs you can switch to other apps. Keep isoboard running. If notifications are allowed, isoboard tells you when the model is ready.

If a download keeps failing, see [model download problems](../../troubleshooting/model-downloads/).

## Where does isoboard store the model files?

<!-- os:macos -->
In `~/Library/Application Support/app.isoboard/models/<model-id>/<revision>/`, next to your configuration file. Downloads come from Hugging Face. Profiles that use the same model share one copy.

## Can I use isoboard offline?

Yes. After the model is downloaded, capture and conversion need no internet connection. Only the first download, and any provider you choose, need one.

## How do I remove a local model?

1. Open the **Profiles** tab and select a Profile that uses the model.
2. Select the trash button (**Remove downloaded model**).
3. Confirm. isoboard deletes the downloaded and partial files.

Profiles that use the model need another download before they work again. Removal is blocked while a capture or a model download is running.

## What license does isOCR use?

isOCR and the other local models ship under their own licenses, separate from the isoboard app license. The model's license file sits in its Hugging Face repository, and the info icon beside the model opens the model page. See [pricing and licensing](../../license/pricing-and-licensing/) for the app.

## Frequently asked questions

**How big is the isOCR download?**

About 1 GB for isOCR1 0.8B.

**Does isOCR send my screenshots anywhere?**

No. isOCR runs on your Mac. Only the model download itself contacts Hugging Face. See [what leaves your Mac](../../privacy/data/).

**Can I use my own model file?**

No. isoboard downloads models from its built-in catalog and has no setting for custom file paths. For other models, run them in Ollama or LM Studio and connect through an [OpenAI-compatible server](../openai-compatible/).

## Related

- [Choose a local model or an AI provider](../choose-a-model/)
- [Profiles](../../use/profiles/)
- [Model download problems](../../troubleshooting/model-downloads/)
