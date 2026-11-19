---
title: Paste results into Word, Docs and other apps
description: Learn how isoboard results paste into Word, Excel, Google Docs, slides and note apps, which apps support rich content, and how to paste plain text.
sidebar:
  label: Paste results
  order: 3
---

isoboard copies each result to your clipboard as rich content, the same kind of formatted content you get when you copy from a web page or a document. How much of it survives depends on the app you paste into. Word and Google Docs keep almost everything, slide apps keep the content but need some rearranging, and some apps keep only the text.

## What is rich clipboard content?

When you copy something formatted, your Mac keeps more than one version of it on the clipboard: a rich version with styling, tables and images, and a plain-text version. The app you paste into picks the richest version it understands.

isoboard puts both versions on the clipboard:

| Version | What it contains | Used by |
|---|---|---|
| Rich content | Text with styling, tables, links and images | Apps that accept formatted content, such as word processors, spreadsheets and many note apps |
| Plain text | The same text without styling. Table cells are separated by spaces and rows by line breaks. | Apps and fields that accept only text, such as terminals, search boxes and some chat apps |

Not every app supports rich content, and the ones that do support it to different degrees. One app may keep tables and images but drop colors, and another may keep everything but need some rearranging.

## Which apps support isoboard results?

These are the apps tested so far. This table is a work in progress and will grow as more apps are tested.

| App | Text styling | Tables | Images | Notes |
|---|:---:|:---:|:---:|---|
| Microsoft Word | ✅ | ✅ | ✅ | For the best result, paste with **Keep Source Formatting**. See [the FAQ below](#frequently-asked-questions). |
| Microsoft Excel | ✅ | ✅ | ✅ | Tables paste into cells. |
| Google Docs | ✅ | ✅ | ✅ | |
| Microsoft PowerPoint | ✅ | ✅ | ✅ | Everything pastes, but you may need to move and resize elements afterward. |
| Google Slides | ✅ | ✅ | ✅ | Everything pastes, but you may need to move and resize elements afterward. |
| Notion | 🟡 | ✅ | ❔ | Some text styling is lost. |
| Apple Notes | 🟡 | ❔ | ❔ | Some text styling is lost. |
| Canva | ❌ | ❌ | ❌ | Does not accept rich content. Text pastes without formatting. |

✅ Supported · 🟡 Partly supported · ❌ Not supported · ❔ Not tested yet

<!-- TODO(fact): Test and add Google Sheets, Numbers, Pages, Keynote, Obsidian, Bear, Gmail, Outlook and Slack. Confirm the "Not tested" cells for Notion and Apple Notes. -->

Is your app missing, or does it behave differently? [Tell us on GitHub](../../community/feedback/) so the table can include it.

## What formatting is kept when I paste?

isoboard keeps the structure a model reads from your capture, then cleans it so it is safe to paste. The destination app then keeps what it supports and ignores the rest.

| Content | What isoboard keeps |
|---|---|
| Text | Headings, paragraphs, bold, italic and other emphasis, lists, quotes and code |
| Tables | Rows, columns, merged cells and captions. Borders are kept where the capture shows them. |
| Colors | Text and background colors, if **Colors** is on in the [Profile](../profiles/) |
| Layout | Alignment, font size, indentation and spacing |
| Links | Web and email links. Other links lose their address and stay as text. |
| Images | Photos and figures in the capture, cropped and added to the result, if **Images** is on |

The plain-text version has no Markdown symbols, table borders or link addresses.

## How do I paste only the text?

1. Open [History](../history/) in the isoboard window.
2. Select the entry, then open the menu next to its copy button.
3. Choose **Copy as plain text**.
4. Paste into the destination.

You can also use your destination app's own paste-as-plain-text option, if it has one. The copy button itself, **Copy result**, restores the rich result.

## How do I paste a result again?

Open [History](../history/) and select **Copy result** on the entry, or choose it from the **History** submenu of the menu bar menu. isoboard does not run the model again. You can also change **Tables**, **Colors** and **Images** on the entry before you copy it.

## Why did my table paste as text?

The destination app accepts only plain text, or it kept only the text from the rich content. Paste into an app that accepts formatted content, such as Word or Google Docs. Check **Tables** in the [Profile](../profiles/). When it is off, isoboard intentionally writes table rows as lines of text with spaces between cells.

## Why are the images missing?

**Images** may be off in the Profile, or the capture contained no figures large enough to keep. Very small images are left out. Some apps also drop embedded images when they paste. See [set up Profiles](../profiles/).

## Frequently asked questions

**How do I get the best results in Microsoft Word?**

Paste as usual, then select the **Paste Options** button that appears next to the pasted content and choose **Keep Source Formatting**. Word's other paste options can drop colors, table styling or images. To make it the default, set pasting from other programs to **Keep Source Formatting** in Word's settings.

**Can I paste a table into Excel or Google Sheets?**

Yes in Excel, where the table pastes into cells. Google Sheets has not been tested yet. If **Tables** is off in the Profile, the table pastes as lines of text instead.

**Does isoboard produce Markdown?**

No. The result is formatted content plus plain text. Editors that convert pasted formatting to Markdown, such as some note-taking apps, can do so. Plain text contains no Markdown symbols.

**Why does the pasted result look different from the capture?**

isoboard recreates the content, so fonts and spacing follow the destination app. Colors, alignment and sizes are kept where the app supports them.

## Related

- [Capture part of your screen](../capture/)
- [History](../history/)
- [Set up Profiles](../profiles/)
