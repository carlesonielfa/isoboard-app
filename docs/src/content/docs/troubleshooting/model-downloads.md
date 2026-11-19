---
title: isoboard can't download the local model
description: Fix isOCR and local model download problems in isoboard. Covers failed and stalled downloads, disk space, verification, loading errors and removal.
sidebar:
  label: Model downloads
  order: 3
---

isoboard downloads a local model such as isOCR™ from Hugging Face, checks it and stores it on your Mac. Downloads fail mostly because of the network or disk space. A failed or paused download keeps its partial files, so you can resume it. Find the message you see below.

## Download failed. Check your connection and disk space, then retry.

You see this message under the model after a download stops with an error. A capture that triggers the download shows "the model download failed; check your connection and try capture again".

**Cause.** The connection dropped, Hugging Face was unreachable, the download stalled, or the disk is full.

**Fix.**

1. Check that your Mac is online and can reach `huggingface.co`. A VPN, proxy or firewall can block it.
2. Make sure you have about 1 GB of free disk space. See [local models](../../models/local-models/) for the requirements.
3. Select **Retry model download** in the **Profiles** tab, or capture again to retry automatically. The download continues from the partial files.

## The download is stuck or very slow

**Cause.** The progress bar stops when the connection stalls. Large files on a slow connection can take a long time.

**Fix.** Select the cancel button (**Cancel download; keep partial files for resume**), wait a moment, then select **Resume model download**. A paused download keeps its partial files, even after you quit and reopen isoboard. Keep isoboard running while it downloads. You can switch to other apps.

## I cancelled the download and want to continue it

Select **Resume model download** in the **Profiles** tab, or **Resume download** in the setup guide. The download continues from where it stopped.

## Verifying never finishes or the model does not become available

After the last byte, isoboard shows **Verifying** while it checks the downloaded files.

**Cause.** A file that fails the check is discarded, so the model never becomes usable. Corrupted data usually comes from an unstable connection. The error shown is the generic "Download failed. Check your connection and disk space, then retry."

**Fix.** Retry the download. If it fails again, remove the model and download it fresh as described below.

## The selected local model is not downloaded

You see "the selected local model is not downloaded" when you capture.

**Cause.** The Profile uses a local model whose files are missing. This happens after you remove the model, or before the first download finishes.

**Fix.** Open the **Profiles** tab, select the Profile and download the model. Other Profiles that use the same model share the download.

## The selected local model could not be loaded

You see "the selected local model could not be loaded".

**Cause.** The model files are present, but isoboard could not start the model. Low free memory or damaged files are possible causes. isOCR needs about 2 GB of free memory.
<!-- TODO(fact): What are the known causes of this error (not enough memory, unsupported hardware, corrupt files)? Confirm before stating a single cause. -->

**Fix.**

1. Close memory-heavy apps and capture again.
2. If it still fails, remove the model and download it again.

## Loading model takes a while on the first capture

The footer and overlay show **Loading model** while the model starts. The first capture after a download, and a capture after a pause, take longer. This is expected. See [local models](../../models/local-models/).

## Wait until capture and model downloads finish

You see one of these messages when you start, change or remove a model:

- "Wait until capture and model downloads finish."
- "Wait until capture finishes."
- "Wait until model resource changes finish."

**Cause.** isoboard blocks changes to downloaded models while a capture or another download is running.

**Fix.** Wait for the capture or download to finish, then try again. If a capture was blocked by a model change, you may see "the inference option model resource changes during capture is not supported by this provider or model". Capture again once the change finishes.

## Local inference is unavailable

You see "Local inference is unavailable in this build." or "Local inference is unavailable."

**Cause.** This build of isoboard does not include the local model runtime, or the Mac is not supported.
<!-- TODO(fact): Which Macs or builds lack local inference? Confirm user-facing wording. -->

**Fix.** Use an [AI provider](../../models/choose-a-model/) instead, or update isoboard following the [install guide](../../getting-started/install/).

## Remove a model and download it again

1. Open the **Profiles** tab and select a Profile that uses the model.
2. Select the trash button (**Remove downloaded model**) and confirm **Delete model**.
3. Download the model again.

Removal deletes the downloaded and partial files. If the button is unavailable, check that no capture or download is running.

### Delete model files by hand

If the app cannot remove a model, quit isoboard and delete the model folder in Finder.

<!-- os:macos -->
Models live in the `models` folder of the isoboard data folder. See [where isoboard stores the model files](../../models/local-models/#where-does-isoboard-store-the-model-files).

## Related

- [Run isOCR and other local models](../../models/local-models/)
- [Choose a local model or an AI provider](../../models/choose-a-model/)
- [Uninstall isoboard](../uninstall/)
