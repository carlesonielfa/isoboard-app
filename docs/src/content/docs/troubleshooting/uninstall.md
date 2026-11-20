---
title: Uninstall isoboard and remove its data
description: Uninstall isoboard from your Mac and delete its settings, History, downloaded models, Keychain items, login item and permissions. Includes older versions.
sidebar:
  label: Uninstall
  order: 6
---

<!-- os:macos -->
Dragging isoboard to the Trash removes the app but leaves its data. To remove everything, turn off **Launch at login**, quit and delete the app, then delete its data folder, its Keychain items and its permissions. This page covers every location, and it is written for macOS.

## Which parts of isoboard stay on my Mac after I delete the app?

| Item | Location | Contains |
|---|---|---|
| Settings and data folder | `~/Library/Application Support/app.isoboard/` | `config.toml`, History database, downloaded models, license records |
| Cache | `~/Library/Caches/app.isoboard/` | A cached copy of the public model catalog |
| Keychain items | Login keychain, service names that start with `app.isoboard` | Provider API keys and the license key |
| Login item | A background item registered with macOS | Starts isoboard at login |
| Permissions | **Privacy & Security** in System Settings | Screen Recording and notifications |

## Uninstall isoboard on macOS

1. Open the **Settings** tab → **General** and turn off **Launch at login**.
2. Choose **Quit isoboard** in the [menu bar](../../use/menu-bar/).
3. Open your **Applications** folder and drag **isoboard** to the Trash.
4. Empty the Trash.

Do these steps in this order. Turning off **Launch at login** while the app still exists removes the login item cleanly.

## Delete the isoboard data folder

Quit isoboard first.

1. In Finder, open the **Go** menu and choose **Go to Folder**.
2. Paste `~/Library/Application Support/app.isoboard/` and press `↩ Return`.
3. Move the **app.isoboard** folder to the Trash.

The folder holds these files and folders:

| Name | What it is |
|---|---|
| `config.toml` | Your settings, Profiles and shortcuts |
| `history.sqlite3` | Your History |
| `models/` | Downloaded local models such as isOCR™, about 1 GB |
| `licensing.json` | The evaluation dates |
| `commercial-license.json` | Cached license rights, not the key itself |

To free disk space and keep everything else, delete only the `models` folder. See [local models](../../models/local-models/). Deleting the folder removes History. If you plan to reinstall, copy `history.sqlite3` and `config.toml` first.

Then remove the cache:

1. Choose **Go to Folder** again and paste `~/Library/Caches/app.isoboard/`.
2. Move the folder to the Trash.
<!-- TODO(fact): Confirm whether web storage also creates ~/Library/WebKit/app.isoboard and ~/Library/HTTPStorages/app.isoboard on a clean install, and whether they hold anything users should remove. -->

## Remove API keys and the license key from the Keychain

isoboard stores each provider API key and the license key in the macOS Keychain, outside the data folder. Deleting the folder does not delete them.

1. Open **Keychain Access** from Applications → Utilities.
2. Select the **login** keychain and the **Passwords** category.
3. Search for `app.isoboard`.
4. Select each item whose name starts with `app.isoboard.managed` and press `⌫ Delete`. Confirm.

You can also remove the license key inside the app before you uninstall. Open the **Settings** tab → **License** and select **Remove license**. This does not cancel the purchase. See [buy and activate a license](../../license/activate/).

:::caution
Revoke API keys you no longer need in your provider account too. Deleting the Keychain item removes the copy on your Mac, not the key itself.
:::

## Remove the login item

Turning off **Launch at login** before you delete the app is the cleanest route. If you already deleted the app:

1. Open **System Settings** → **General** → **Login Items & Extensions**.
2. Look for **isoboard** under **Open at Login** and **Allow in the Background**, and turn it off or remove it.
3. If **isoboard** is missing from the list but the app still starts at login, delete `~/Library/LaunchAgents/isoboard.plist`.

## Remove the Screen Recording and notification permissions

1. Open **System Settings** → **Privacy & Security** → **Screen Recording**.
2. Select **isoboard** and click **−**.
3. Open **System Settings** → **Notifications**, select **isoboard** and turn off **Allow Notifications**, if it is still listed.

To reset the permission from Terminal instead, run `tccutil reset ScreenCapture app.isoboard`.

## Remove data from earlier versions

Earlier pre-release builds used the bundle identifier `com.carlesonielfa.isoboard`. The current identifier is `app.isoboard`. If you used an earlier build, also check for:

- `~/Library/Application Support/com.carlesonielfa.isoboard/`
- `~/Library/Caches/com.carlesonielfa.isoboard/`
- Keychain items that start with `com.carlesonielfa.isoboard`
- A **Screen Recording** entry for the earlier app

isoboard does not move data from the earlier identifier, so these items stay until you delete them.

## Frequently asked questions

**Does uninstalling isoboard cancel my license?**

No. Your license belongs to your Polar purchase, not to the app. You can install isoboard again and enter the same key. See [buy and activate a license](../../license/activate/).

**Does uninstalling delete my captures?**

Your History holds results and small thumbnails in `history.sqlite3`, and the original screenshots of your most recent captures in `history-originals/`. Deleting the data folder removes both. See [what leaves your Mac](../../privacy/data/).

## Related

- [Reset isoboard settings](../reset-configuration/)
- [Install isoboard](../../getting-started/install/)
- [What leaves your Mac](../../privacy/data/)
