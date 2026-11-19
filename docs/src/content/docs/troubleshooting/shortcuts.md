---
title: isoboard keyboard shortcut doesn't work
description: Fix isoboard shortcuts that do nothing, will not record or report a conflict. Covers taken key combinations, recording errors and invalid config values.
sidebar:
  label: Shortcuts
  order: 2
---

<!-- os:macos -->
A shortcut can fail because another app already uses it, because the recorder rejected the key, or because the saved value in the configuration file is invalid. The **Shortcuts** tab and the footer of the settings window show the error. Find your message below. The defaults and the recording steps are in the [keyboard shortcuts reference](../../reference/keyboard-shortcuts/).

## The capture shortcut does nothing

**Cause.** There are four common reasons:

- Setup is incomplete, so capture is disabled. The footer shows a label such as **Screen access required** or **Profile needs attention**.
- The shortcut did not register. The footer shows **Shortcut unavailable**.
- Another capture is already running.
- Another app took the same key combination.

**Fix.**

1. Hover the footer status and read the detail. Fix what it names. See [fix capture and permission problems](../capture-and-permissions/).
2. Open the **Shortcuts** tab and record a different combination.
3. Quit apps that register global shortcuts, then try the original combination again.

## could not register shortcut

You see an error like this when you record a shortcut that is taken:

> could not register shortcut "CommandOrControl+Shift+KeyK": ... Choose another shortcut.

**Cause.** macOS or another running app already owns that key combination. isoboard keeps your previous shortcut active.

**Fix.** Record a different combination. Combinations with two modifiers such as `⌘ Cmd` + `⌥ Option` are less likely to collide with other apps.

## duplicate capture shortcut

You see `duplicate capture shortcut "..."`.

**Cause.** Two capture shortcuts use the same key combination. Each capture shortcut, including each [Profile](../../use/profiles/) shortcut, needs its own combination.

**Fix.** Record a different combination for one of them.

## open shortcut conflicts with a capture shortcut

You see `open shortcut "..." conflicts with a capture shortcut`.

**Cause.** The **Open isoboard window** shortcut matches one of the capture shortcuts.

**Fix.** Record a different combination for one of them.

## That key is not available for a global shortcut

You see "That key is not available for a global shortcut. Try another key." under the shortcut field.

**Cause.** The recorder could not turn the key you pressed into a shortcut. Some keys are not supported, for example the extra key on ISO keyboards next to the left `⇧ Shift` key.

**Fix.** Press a different key together with at least one modifier. A shortcut needs a modifier such as `⌘ Cmd`, `⌃ Control`, `⌥ Option` or `⇧ Shift`.

## Release the keys, then press the shortcut once

You see this message while recording.

**Cause.** You held the keys down, so the recorder received repeated key events.

**Fix.** Release all keys, then press the combination once.

## The recorder never starts or stops right away

You see one of these messages:

- "Keep the isoboard window focused to record a shortcut."
- "Could not restore the active global shortcut. Try Escape and record again."

**Cause.** Recording works only while the isoboard window is in front. Switching to another app or clicking elsewhere cancels the recording. If isoboard cannot restore the shortcuts afterward, it shows the second message.

**Fix.** Keep the isoboard window in front while you record. Press `⎋ Escape` to cancel, then select the shortcut field and record again. If the message persists, quit and reopen isoboard.

## invalid capture shortcut after editing config.toml

You see an error such as `invalid capture shortcut "..."` or `invalid open shortcut "..."`.

**Cause.** The value in `config.toml` is not a valid shortcut. isoboard rejects the edit and keeps the last valid configuration.

**Fix.** Correct the value using the syntax in the [configuration file reference](../../reference/configuration-file/#how-are-shortcuts-written), or record the shortcut in the **Shortcuts** tab. If you cannot find the problem, see [reset isoboard settings](../reset-configuration/).

## The default capture shortcut is gone

**Cause.** The default capture shortcut cannot be deleted. If it is missing, it was removed from `config.toml` by hand. isoboard then reports "capture.bindings must contain the default binding" or "capture.bindings must contain exactly one default binding".

**Fix.** Restore the entry in `config.toml`, or remove the `[capture]` section and let isoboard recreate the default. See [reset isoboard settings](../reset-configuration/).

## Related

- [Keyboard shortcuts reference](../../reference/keyboard-shortcuts/)
- [Capture a region](../../use/capture/)
- [Fix capture and permission problems](../capture-and-permissions/)
- [Configuration file reference](../../reference/configuration-file/)
