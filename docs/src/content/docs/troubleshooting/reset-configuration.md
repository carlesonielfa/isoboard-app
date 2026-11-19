---
title: Reset isoboard settings to the defaults
description: Reset isoboard settings by moving config.toml aside. Fix a config that will not load, keep your History and models, and learn what the reset removes.
sidebar:
  label: Reset settings
  order: 5
---

<!-- os:macos -->
To reset isoboard, quit the app, move `config.toml` out of its folder and open the app again. isoboard creates a fresh file with the defaults and runs setup again. The reset keeps your History, downloaded models and saved Keychain items. This page is written for macOS.

## When should I reset settings?

Reset when isoboard shows one of these errors and you cannot fix the file by hand:

- `could not parse config <path>: ...`
- `unsupported config version N; expected 1`
- `unknown field` followed by a name, which means the file has a setting that isoboard does not know
- `active profile "..." does not exist`
- `duplicate profile name "..."`
- `processing must contain at least one profile`

isoboard keeps using the last valid configuration while the file on disk is invalid. At startup with an invalid file it runs with the defaults and does not overwrite your file. The error stays visible until the file is valid again. See the [configuration file reference](../../reference/configuration-file/) for what each value accepts.

Settings from early pre-release builds can also stop loading. isoboard does not migrate those older files.

## Reset all settings

1. Quit isoboard. Choose **Quit isoboard** in the [menu bar](../../use/menu-bar/).
2. In Finder, open the **Go** menu, choose **Go to Folder** and paste `~/Library/Application Support/app.isoboard/`.
3. Move `config.toml` to your Desktop as a backup. Move only this one file.
4. Open isoboard again. It creates a new `config.toml` and opens the setup guide.
5. Finish setup. See [your first capture](../../getting-started/first-capture/).

An empty `config.toml` is also a valid reset.

## What does a reset change?

| Reset | Kept |
|---|---|
| Profiles, including Additional Rules and model choices | History |
| Capture and open shortcuts | Downloaded local models |
| Display mode (Basic, Advanced, Debug), **Play sounds** and overlay position | Your license and evaluation records |
| Provider connections | API keys in the Keychain, which are no longer connected |

The provider connections live in `config.toml`, so after a reset you enter your API keys again on the **Providers** tab. Connecting a provider creates its starting Profiles again. The old Keychain items stay until you delete them. See [uninstall isoboard](../uninstall/#remove-api-keys-and-the-license-key-from-the-keychain).

Do not delete the other files in the folder to reset settings. They hold History, models and license data. Moving only `config.toml` leaves them untouched.

## Reset only the shortcuts

1. Quit isoboard.
2. Open `config.toml` in a text editor.
3. Delete the `[capture]` and `[shortcuts]` sections, including any `[[capture.bindings]]` entries.
4. Save the file and open isoboard. It loads the default shortcuts.

The defaults are `⌘ Cmd` + `⇧ Shift` + `2` for capture and `⌘ Cmd` + `⌥ Option` + `Z` for **Open isoboard window**. See [keyboard shortcuts](../../reference/keyboard-shortcuts/).

## Restart the setup guide without resetting

If setup is complete and you only want to see it again, open the **Settings** tab → **General** and select **Restart initial setup**. This changes no settings.

## Reset the Screen Recording permission

A settings reset does not touch macOS permissions. See [fix capture and permission problems](../capture-and-permissions/#grant-screen-recording-permission-on-macos).

## Frequently asked questions

**Does resetting delete my History?**

No. History lives in its own database next to the configuration file. Moving `config.toml` does not touch it. See [History](../../use/history/).

**Do I lose my downloaded isOCR model?**

No. Models sit in a `models` folder beside `config.toml`, and a reset leaves them. After the reset, choose the model in a Profile again. See [local models](../../models/local-models/).

## Related

- [Configuration file reference](../../reference/configuration-file/)
- [Settings reference](../../reference/settings/)
- [Uninstall isoboard](../uninstall/)
