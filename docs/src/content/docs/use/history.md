---
title: Find and copy past captures in History
description: isoboard History keeps up to 1,000 results and your recent failed attempts. Copy a result again as rich or plain text, adjust Tables, Colors and Images, or delete one.
sidebar:
  label: History
  order: 4
---

History lists your recent isoboard captures. Select an entry to preview its result, then copy it back to the clipboard as rich content or plain text without running the model again. isoboard keeps up to 1,000 successful results and the newest 20 failed attempts on your Mac.

## How do I open History?

- Open the **History** tab in the isoboard window. It is the first tab.
- Choose **View all history** from the **History** submenu of the menu bar menu.
- Click the capture overlay while it is showing, and isoboard opens that capture's entry.

<!-- os:macos -->
The **History** submenu in the menu bar shows your five most recent successful results. Select one to copy it back to the clipboard. See [use isoboard from the menu bar](../menu-bar/).

## What does History save?

For each capture History stores:

- The result: rich content and plain text.
- A small thumbnail of the source image.
- For your 50 most recent captures, the original full-resolution screenshot.
- The time and the Profile name.
- A summary of the settings used: the model for each pass, the Tables, Colors and Images options, and whether Additional Rules ran.

History does not store your API keys, provider endpoints or the text of your Additional Rules. To stop keeping original screenshots, turn off **Keep original screenshots** in Settings > History. This also deletes the screenshots already kept. A capture that is still processing shows as a pending entry with a spinner. It becomes a result when the clipboard copy succeeds, or a failed attempt if it does not.

## How long does History keep entries?

| Item | Limit |
|---|---|
| Successful results | Up to the newest 1,000, within 500 MB of storage |
| Failed attempts | The newest 20, counted separately |
| Original screenshots | The newest 50, within 1 GB, counted separately |

When you exceed a limit, isoboard removes the oldest entries. An entry whose original screenshot expires keeps its result and thumbnail. You cannot change these limits.

## How do I find an entry?

Type in the search field above the list. History shows entries whose text or Profile name contains words starting with what you typed, and highlights the matches. Upper and lower case and accents do not matter. Select the filter button inside the search field to show only results or failed attempts, one Profile, results that contain tables, images, styling or code, or a range of days. Choosing several kinds of content shows results that contain all of them. Results saved before you updated isoboard are not tagged with their content, so a content filter leaves them out. On the calendar, select the first day and then the last; select the same day twice for a single day. **Clear filters** in the menu removes them all but keeps your search text. The **×** button in the search field clears the search text and every filter at once. A capture that is still processing appears only while no search or filter is active.

## How do I copy a result again?

Each entry has a copy button.

1. Select **Copy result** to put the rich result on the clipboard. The button shows a check mark when done.
2. To copy plain text, open the menu next to the button and choose **Copy as plain text**.

Neither action runs the model again. See [paste results into other apps](../paste/).

## How do I change Tables, Colors or Images on a saved result?

Select a successful entry. Its details list **Options** with **Tables**, **Colors** and **Images** switches, set to the values used for the capture.

Turning a switch off updates the preview and both copy actions at once. isoboard applies the change to the saved result and does not call the model. Selecting another entry and coming back resets the switches, and the saved result itself never changes.

You can turn an option back on only if the model produced that content. An option that was off during the capture usually cannot be restored, because the content was never kept. If every option is off and nothing is left, copying is disabled.

## What do I see for a failed capture?

A failed entry shows **Capture failed**, the error message and an error code. Select **Copy error details** to copy the time, source, Profile, code and message as plain text, for example to share with support. Failed entries have no result to copy. See [fix AI provider errors](../../troubleshooting/provider-errors/).

## How do I delete an entry?

1. Open the menu next to the entry's copy button.
2. Choose **Delete**.

Deleting removes the entry from isoboard. It is not a secure erase. History has no clear-all action, so delete entries one at a time.

You can see how History data is stored in [what leaves your Mac](../../privacy/data/).

## What extra details does History show in Advanced mode?

**Advanced** mode adds **Conversion mode** and **Image size** to an entry's details. In Basic mode, **Additional Rules** appears in the details only when rules were applied. The footer switch changes the mode.

## Frequently asked questions

**Does History keep my screenshots?**

No. It keeps the converted result and a small thumbnail of the source, not the original image. History is stored locally and can contain sensitive text, so delete entries you no longer need.

**Can I search History?**

Yes. See [how do I find an entry?](#how-do-i-find-an-entry). Entries are listed newest first, and older ones load as you scroll.

**Why is an old result missing?**

History keeps up to 1,000 successful results within 500 MB and removes the oldest entries first. Long or image-heavy results use more of that space.

## Related

- [Paste results into other apps](../paste/)
- [Capture part of your screen](../capture/)
- [What leaves your Mac](../../privacy/data/)
