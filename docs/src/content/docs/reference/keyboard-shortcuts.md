---
title: isoboard keyboard shortcuts
description: Default isoboard shortcuts for capturing a region and opening the window, how to record your own, how Profile shortcuts work and the keys used inside the app.
sidebar:
  label: Keyboard shortcuts
  order: 2
---

<!-- os:macos -->
isoboard has two default global shortcuts. `⌘ Cmd` + `⇧ Shift` + `2` captures a screen region, and `⌘ Cmd` + `⌥ Option` + `Z` opens the isoboard window. You can change both and add one capture shortcut for each Profile. Global shortcuts work while any app is in front. This page is written for macOS.

## What are the default shortcuts?

| Action | Default | What it does |
|---|---|---|
| **Capture region** | `⌘ Cmd` + `⇧ Shift` + `2` | Starts a region capture with the active Profile. |
| **Open isoboard window** | `⌘ Cmd` + `⌥ Option` + `Z` | Shows and focuses the settings window. |

The **Capture region** shortcut cannot be deleted, and it always follows the active Profile. The [menu bar](../../use/menu-bar/) menu shows both shortcuts beside **Capture region** and **Open isoboard**. See [capture a region](../../use/capture/) for what happens after you press the shortcut.

## How do I change a shortcut?

1. Open the **Shortcuts** tab.
2. Select the shortcut field you want to change. It shows **Press a shortcut…**.
3. Press the new combination once. It must include at least one modifier key.
4. isoboard saves it right away. Press `⎋ Escape` to cancel, or click away from the field.

Keep the isoboard window in front while you record. If a key combination is taken, isoboard shows an error and keeps the previous shortcut. See [fix keyboard shortcut problems](../../troubleshooting/shortcuts/).

### Which keys can I use?

- Use at least one modifier from `⌘ Cmd`, `⌃ Control`, `⌥ Option` and `⇧ Shift`.
- Letters, digits, function keys, arrow keys, `Space` and most other keys work.
- Some keys are not available, for example the extra key on ISO keyboards beside the left `⇧ Shift`.
- Capture shortcuts and the open-window shortcut must all be different.

## How do Profile shortcuts work?

A Profile shortcut captures a region with one specific [Profile](../../use/profiles/), whichever Profile is active. For example, you can keep a translation Profile on one shortcut and use the default shortcut for plain extraction.

The **Profile shortcuts** list appears in the **Shortcuts** tab when you have more than one Profile.

1. Under **Profile shortcuts**, choose a Profile in the empty row.
2. Select its shortcut field and press the combination.
3. To change the Profile later, choose another Profile in the row's dropdown. You do not need to record the shortcut again.
4. To delete a Profile shortcut, select its trash button.

If you delete a Profile, any shortcut that uses it is removed.

The shortcut picks its Profile when you press it. See [Profiles](../../use/profiles/).

## What keys does isoboard use inside its windows?

| Key | Where | What it does |
|---|---|---|
| `⎋ Escape` | Region picker | Cancels the capture. |
| `⎋ Escape` | A shortcut field while recording | Cancels recording and keeps the current shortcut. |
| `⎋ Escape` | Model picker list | Closes the list and restores the current model. |
| `↩ Return` | Model picker | Uses the text you typed as the model ID. |
| `←` `→` `↑` `↓` | **Position** grid for the capture overlay | Moves between the nine positions. |
| `↖ Home` and `↘ End` | **Position** grid | Jumps to the first and last position. |
| `↑` and `↓` | **Position** dropdown button | Opens the grid. |
| `←` and `→` | Settings tabs | Moves between tabs. |
| `↑` and `↓` | **History** and **Profiles** lists | Moves between entries. |
| `⇥ Tab` | Settings window | Moves between tabs, fields and buttons. |

## Where are shortcuts stored?

<!-- os:macos -->
In `~/Library/Application Support/app.isoboard/config.toml`. The recorder writes names such as `CommandOrControl+Shift+Digit2`. See [how shortcuts are written](../configuration-file/#how-are-shortcuts-written) if you want to edit them by hand.

## Frequently asked questions

**What is the default screenshot shortcut in isoboard?**

`⌘ Cmd` + `⇧ Shift` + `2` captures a region. `⌘ Cmd` + `⌥ Option` + `Z` opens the isoboard window. You can change both in the **Shortcuts** tab.

**Can I use different shortcuts for different Profiles?**

Yes. Add a Profile shortcut for each Profile you want to reach directly. The default **Capture region** shortcut always uses the active Profile.

**Why does my shortcut not work?**

Another app may own the same combination, or setup may be incomplete. See [fix keyboard shortcut problems](../../troubleshooting/shortcuts/).

## Related

- [Capture a region](../../use/capture/)
- [Profiles](../../use/profiles/)
- [Fix keyboard shortcut problems](../../troubleshooting/shortcuts/)
- [Settings reference](../settings/)
