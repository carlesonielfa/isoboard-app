---
title: isoboard can't capture the screen
description: Fix isoboard capture problems on a Mac. Covers the Screen Recording permission, a required restart, blank captures and tiny selections.
sidebar:
  label: Capture and permissions
  order: 1
---

<!-- os:macos -->
Most capture problems in isoboard come from the Screen Recording permission, an unfinished setup or a selection that is too small. The footer of the settings window names the next step. Find the message you see below, then follow its fix. This page is written for macOS.

## Screen Recording permission is required

You see this error after you try to capture:

> Screen Recording permission is required. Enable isoboard in System Settings > Privacy & Security > Screen Recording.

The footer shows **Screen access required**. The failed capture is labeled **Screen access needed**.

**Cause.** macOS has not allowed isoboard to record the screen, or you turned the permission off later.

**Fix.** Follow [Grant Screen Recording permission on macOS](#grant-screen-recording-permission-on-macos).

## The footer says Restart required

The footer shows **Restart required** with the detail "Restart isoboard to finish enabling Screen Recording."

**Cause.** You enabled the permission in System Settings, but macOS applies it only to a new process.

**Fix.** Select **Restart isoboard** on the setup screen, or choose **Quit isoboard** in the [menu bar](../../use/menu-bar/) and open isoboard again. The setup screen reopens if it was not finished.

## Grant Screen Recording permission on macOS

1. Open **System Settings** and go to **Privacy & Security** → **Screen Recording**. In setup, select **Open System Settings** to land there directly.
2. Turn on the switch next to **isoboard**. If isoboard is not in the list, select **+**, choose isoboard from your Applications folder and enable it.
3. Return to isoboard. It rechecks access when its window regains focus.
4. If the footer then shows **Restart required**, restart isoboard as described above.

If the permission stays off after these steps, reset it and grant it again:

1. Quit isoboard.
2. Run this command in Terminal:

   ```sh
   tccutil reset ScreenCapture app.isoboard
   ```

3. Open isoboard and grant the permission when macOS asks.

The [permissions page](../../getting-started/permissions/) covers the same steps for first-time setup.

## The footer says Setup incomplete or the Capture button is disabled

Capture stays disabled until setup is complete. Hover or focus the footer status to read the detail. These are the messages you can see:

| Footer label | Detail or cause | Fix |
|---|---|---|
| Screen access required | "Allow Screen Recording access to capture a region." | Grant the permission above. |
| Restart required | "Restart isoboard to finish enabling Screen Recording." | Restart isoboard. |
| Profile needs attention | The active [Profile](../../use/profiles/) has no model, its model is unavailable or its provider is not connected. | Open the **Profiles** tab and fix the model, or open the **Providers** tab. See [fix AI provider errors](../provider-errors/). |
| Shortcut unavailable | A capture shortcut could not register. | See [fix keyboard shortcut problems](../shortcuts/). |
| Checking… | isoboard is still loading settings or checking access. | Wait a moment. |
| Setup incomplete | The setup guide is open and a step is unfinished. | Finish the setup guide. |

For a local model, the Profile detail reads "Download the selected local model to finish setup." See [model download problems](../model-downloads/).

## I selected a region and nothing happened

**Cause.** isoboard treats a very small selection as an accidental drag and cancels it. A small cancelled capture shows no error, produces no clipboard result and creates no History entry.

**Fix.** Select a larger region.

## The capture is blank or black

**Cause.** Some apps and protected content, such as streaming video, block screen capture. macOS then gives isoboard a blank area, so there is nothing to read.

**Fix.** Capture a different window or another source of the same content.

## The capture is cancelled when I press Escape

This is expected. Pressing **Escape** in the macOS region picker cancels the capture. isoboard shows "screen region capture cancelled" in its status and does nothing else.

## The result could not be copied to the clipboard

You see one of these errors:

- "The inference result could not be prepared for the clipboard."
- "The inference result could not be written to the clipboard."

**Cause.** The model returned content that isoboard cannot turn into safe clipboard data, or macOS refused the clipboard write.

**Fix.** Capture again. If it keeps failing, open the failed entry in **History**, select **Copy error details** and send the text to support. See [History](../../use/history/).

## Capture setup is incomplete or capture is busy

- "Capture setup is incomplete." means you triggered a capture before setup finished. Check the footer.
- "capture is busy" means another capture or file conversion is still running. Wait for it to finish. A capture can also wait on a model that is downloading or loading.

## Related

- [Grant macOS permissions](../../getting-started/permissions/)
- [Capture a region](../../use/capture/)
- [Fix keyboard shortcut problems](../shortcuts/)
- [Fix AI provider errors](../provider-errors/)
