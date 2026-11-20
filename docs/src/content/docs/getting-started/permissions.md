---
title: Grant macOS permissions for isoboard
description: isoboard needs Screen Recording permission to capture your screen. Notifications and Launch at login are optional. Learn how to allow each one on macOS.
sidebar:
  label: Permissions
  order: 3
---

<!-- os:macos: this page describes macOS permissions. Add Windows and Linux sections beside the macOS headings. -->

isoboard needs one permission: Screen Recording, so it can capture the region you select. Notifications, which only announce finished model downloads, and Launch at login are optional. The setup guide asks for Screen Recording before anything else and enables **Continue** once macOS reports access.

## Which permissions does isoboard use?

| Permission | Required | What it is for |
|---|---|---|
| Screen Recording | Yes, for screen capture | Lets isoboard capture the screen region you select. |
| Notifications | No | isoboard only notifies you when a model download finishes. |
| Launch at login | No, off by default | Starts isoboard in the menu bar when you sign in. |

The global shortcuts need no Accessibility or Input Monitoring permission. isoboard does not listen to your keystrokes.

## Grant Screen Recording permission on macOS

<!-- os:macos -->
The first setup step is called **Allow permissions**. If you skipped it or need to repeat it, open the **Settings** tab and select **Restart initial setup** below **About**.

1. In the **Screen Recording** card, select **Ask macOS for access**. macOS shows its own prompt.
2. In the prompt, choose to open System Settings. You can also select **Open System Settings** in the card.
3. In **System Settings** → **Privacy & Security** → **Screen Recording**, turn on **isoboard**. On newer macOS versions this list is named **Screen & System Audio Recording**.
4. Return to isoboard. It detects the change on its own, and the card shows **Allowed**.
5. Select **Continue**.

The card shows the current state: **Not allowed**, **Waiting for access**, **Allowed** or **Restart needed**.

:::note
isoboard asks macOS for access without capturing anything. It reads your screen only when you start a capture.
:::

### What if the card says Restart needed?

macOS sometimes saves the permission but does not apply it to the running app. Select **Restart isoboard**. macOS may also offer **Quit & Reopen** itself, which works the same way. isoboard reopens its settings window after either one, so you can continue.

### How do I change or remove Screen Recording permission later?

Open **System Settings** → **Privacy & Security** → **Screen Recording** and turn **isoboard** on or off. Without the permission, capture is unavailable and isoboard reports that screen access is needed, for example in the menu bar status. If captures still fail after you turn it on, see [capture and permission problems](../../troubleshooting/capture-and-permissions/).

## Allow notifications on macOS

<!-- os:macos -->
Notifications are optional. isoboard uses them only to tell you when a model download is ready. Declining never blocks **Continue**.

1. In the **Notifications** card of the setup guide, select **Enable notifications**.
2. Allow the macOS prompt.

If you declined earlier, select **Open System Settings** in the card, then allow **isoboard** under **Notifications**. isoboard detects the change when you return.

Without notifications, a model download still finishes. Keep isoboard running while it downloads and check the footer for progress. See [local models](../../models/local-models/).

## Launch at login on macOS

<!-- os:macos -->
Launch at login is off by default.

1. Open the **Settings** tab → **General**.
2. Turn on **Launch at login**.

isoboard then starts in the menu bar when you sign in, without opening its window. The switch follows macOS login items.

## Frequently asked questions

**Why does isoboard need Screen Recording permission?**

macOS requires it for any app that captures the contents of other windows. isoboard uses it to read the region you select when you start a capture.

**Does isoboard record my screen all the time?**

No. isoboard captures only the region you select, and only after you start a capture. See [what leaves your Mac](../../privacy/data/) for what happens to that image.

## Related

- [Make your first capture](../first-capture/)
- [Capture and permission problems](../../troubleshooting/capture-and-permissions/)
- [What leaves your Mac](../../privacy/data/)
