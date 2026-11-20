---
title: isoboard settings reference
description: Every control in isoboard's Settings tab and the footer display mode switch, with its default and effect. Covers Basic, Advanced and Debug modes.
sidebar:
  label: Settings
  order: 1
---

<!-- os:macos -->
isoboard's settings window has five tabs: **History**, **Shortcuts**, **Profiles**, **Providers** and **Settings**. This page lists the controls on the **Settings** tab, the **Basic**, **Advanced** and **Debug** switch in the footer, and the controls that Advanced mode reveals. Changes save as soon as you make them. This page is written for macOS.

## Which tab holds which setting?

| Tab | What you set there | Owning page |
|---|---|---|
| **History** | Browse, copy and delete past results | [History](../../use/history/) |
| **Shortcuts** | Capture shortcuts and the open-window shortcut | [Keyboard shortcuts](../keyboard-shortcuts/) |
| **Profiles** | Model, conversion options and Additional Rules | [Profiles](../../use/profiles/) |
| **Providers** | API keys and endpoints | [Choose a model](../../models/choose-a-model/) |
| **Settings** | General, capture status, history storage, license and about | This page |

## What does the Settings tab contain?

### General

| Setting | What it does | Default |
|---|---|---|
| **Play sounds** | Plays short sounds when region selection starts, when the image is ready, when the result reaches the clipboard and when an error occurs. Turn it off to silence all of them, including errors. | On |
| **Launch at login** | Starts isoboard when you log in to your Mac. A login launch stays in the menu bar and does not open the settings window. | Off |

### Capture status

The capture status is the small overlay that appears on the display where you finish selecting. It shows processing, success and failure.

| Setting | What it does | Default |
|---|---|---|
| **Show capture status** | Shows the overlay after each capture. When it is off, the menu bar icon, sounds and History still report each result. | On |
| **Position** | Chooses where the overlay appears. There are nine positions in a 3 × 3 grid: **Top left**, **Top center**, **Top right**, **Middle left**, **Center**, **Middle right**, **Bottom left**, **Bottom center** and **Bottom right**. It is unavailable while **Show capture status** is off. | **Bottom center** |

### History

| Setting | What it does | Default |
|---|---|---|
| **Keep original screenshots** | Keeps the full-resolution image of your most recent captures on this Mac. Turning it off deletes the ones already kept. Select the label to show this note. | On |

### License

The **License** section shows your license status and the controls to buy, activate or remove a key.

| Control | What it does |
|---|---|
| Status line | Shows "Commercial trial ends (date). Personal use is free.", "Free for personal use. License required for commercial use." (with **Free** in a heart badge) or "Licensed for personal and commercial use." |
| **Continue without a license key** | Starts the seven-day evaluation. It appears only before you choose. |
| **Get a license** | Opens Polar checkout in your browser. |
| **License key** and the activate button | Validates a key you paste. |
| **Remove license** | Deletes the saved key from this Mac. It does not cancel the purchase. |
| **Email support** | Opens an email draft to support@isoboard.app. It appears once a license is active. |
| **Retry** | Checks the license again after an error. |
| **License terms**, **Commercial terms**, **Privacy** | Open the bundled documents. |

See [pricing and licensing](../../license/pricing-and-licensing/) and [buy and activate a license](../../license/activate/).

### About

| Item | What it shows |
|---|---|
| **Version** | The installed isoboard version. |
| **Check for updates** | Checks for a newer isoboard release now. When an update has downloaded, it becomes **Restart to update**. |
| **What's new** | Release notes, newest first. The installed version is marked **Current**. After a check, the newest version is marked **Latest**, including versions you have not installed yet. |
| **Check for updates automatically** | Checks shortly after launch and every six hours, and downloads updates in the background. isoboard asks before it restarts. On by default. |
| **Configuration file** | Select the label to show the full path to `config.toml`, which you can select and copy. **Show file** reveals it in Finder. See the [configuration file reference](../configuration-file/). |
| **Third-party notices**, **MPL source code** | Open the bundled notices and source-access instructions in your default text viewer. |
| **Restart initial setup** | Shows the setup guide again. It changes no settings. The setup guide is shown while setup is incomplete, and **Set up later** hides it for the current session. |

## What do Basic, Advanced and Debug do?

The footer switch is labeled **Display mode**. It sets how many controls the settings window shows. It applies to the whole window. The default is **Basic**.

| Mode | For | What changes |
|---|---|---|
| **Basic** | Everyday use. | Hides expert controls. This is the default. |
| **Advanced** | Tuning how a model is called and how results are converted. | Shows the controls listed below. |
| **Debug** | Diagnosing problems with developer tools. | Includes everything in Advanced and adds developer-only tools that are not covered in the user documentation. |



### What does Advanced mode add?

| Where | Control | What it does |
|---|---|---|
| **Profiles** tab, **Conversion** | **Conversion settings application** (Advanced mode) | Chooses how the **Tables**, **Colors** and **Images** options apply. The options are **Automatic — Prompt + cleanup**, **Prompt only**, **Cleanup only** and **Prompt + cleanup**. Automatic is the default. |
| **Profiles** tab, **Conversion** | **Conversion prompt template** (Advanced mode) | Shows the bundled prompt for editing. A custom template replaces the bundled prompt completely. **Reset to default** restores it. |
| **Profiles** tab, **Rules** | **Postprocessing prompt template** (Advanced mode) | The same override for the separate rules step. It appears only when Additional Rules use a separate pass. |
| **Profiles** tab, **Model** | **Temperature** (Advanced mode) | Sets the sampling temperature from 0 to 2. Empty uses the provider default. It is hidden for models that do not support it. |
| **Profiles** tab, **Model** | **Max output tokens** (Advanced mode) | Limits the length of the reply. Empty uses the provider default. |
| **Profiles** tab, **Model** | **Request timeout (seconds)** (Advanced mode) | Sets how long to wait for the whole response, Empty uses 120 seconds. |
| **History** tab, entry details | **Conversion mode** (Advanced mode) | Shows how the conversion options were applied. |
| **History** tab, entry details | **Additional Rules** row (Advanced mode) | Shown even when the entry used no rules. Basic mode hides it when empty. |
| **History** tab, entry details | Original image size (Advanced mode) | Shows the size of the captured image. |

The **Reasoning** setting in the **Model** section is available in every mode. See [Profiles](../../use/profiles/).

:::note
If a Profile has an invalid **Temperature**, **Max output tokens** or **Request timeout (seconds)** value and you are in Basic mode, the editor shows the error and asks you to enable Advanced settings to edit it.
:::

## Where are settings stored?

<!-- os:macos -->
Most settings live in `~/Library/Application Support/app.isoboard/config.toml`. **Launch at login** is registered with macOS instead. API keys and the license key live in the macOS Keychain. See the [configuration file reference](../configuration-file/) and [what leaves your Mac](../../privacy/data/).

## Frequently asked questions

**Is there a setting to change how many History entries are kept?**

No. The History limits are fixed. See [History](../../use/history/).

**How do I turn off sounds?**

Turn off **Play sounds** in the **Settings** tab → **General**. It silences every sound, including error sounds.

**Where do I change the capture shortcut?**

Open the **Shortcuts** tab and select the shortcut field. See [keyboard shortcuts](../keyboard-shortcuts/).

## Related

- [Keyboard shortcuts](../keyboard-shortcuts/)
- [Configuration file reference](../configuration-file/)
- [Profiles](../../use/profiles/)
- [Reset isoboard settings](../../troubleshooting/reset-configuration/)
