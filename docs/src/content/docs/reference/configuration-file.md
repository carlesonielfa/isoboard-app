---
title: isoboard config.toml reference
description: "Reference for isoboard's config.toml: its location, every section and field, defaults, validation errors, and what happens when you edit it while the app runs."
sidebar:
  label: Configuration file
  order: 3
---

<!-- os:macos -->
isoboard stores its settings in a TOML file named `config.toml`. Most people never need to edit it, because every setting has a control in the app. Advanced users can read this page to edit the file by hand. The file lives in `~/Library/Application Support/app.isoboard/`, and isoboard reloads it when it changes. This page is written for macOS.

:::caution
Keep a backup before you edit. The app rewrites the whole file whenever you change a setting in its window, so your comments and key order are not preserved.
:::

## Where is the configuration file?

<!-- os:macos -->
`~/Library/Application Support/app.isoboard/config.toml`. The **Settings** tab → **About** shows the full path under **Configuration file**. isoboard creates the file with the defaults on first launch.

Other data sits in the same folder, next to `config.toml`:

| Name | What it holds |
|---|---|
| `models/` | Downloaded local models |
| `history.sqlite3` | History |
| `licensing.json`, `commercial-license.json` | Evaluation dates and cached license rights |

API keys and the license key are never in this file. They are in the macOS Keychain. See [what leaves your Mac](../../privacy/data/). Developers building isoboard from source use a different location. See the [developer documentation](../../developers/configuration/).

## What does the default file look like?

An empty file, a file containing only `version = 1` and a file with omitted sections or fields all load these defaults:

```toml
version = 1

[capture]
overlay_position = "bottom_center"

[[capture.bindings]]
id = "default"
shortcut = "CommandOrControl+Shift+2"

[shortcuts]
open = "CommandOrControl+Alt+KeyZ"

[ui]
mode = "basic"
sounds_enabled = true

[updates]
check_automatically = true

[history]
keep_originals = true

[processing]
active_profile = "default"

[processing.profiles.default]
name = "Default"
rules_model_mode = "same"
additional_rules = ""
additional_rules_separate_pass = false

[processing.profiles.default.conversion]
tables = true
colors = true
images = true

[inference]
provisioned_providers = []

[inference.providers]
```

## How does isoboard handle edits while it runs?

- isoboard watches the folder and reloads `config.toml` shortly after a change.
- A valid change takes effect right away, including shortcuts and Profiles.
- An invalid change is rejected. isoboard shows the error and keeps the last valid configuration. Fix the file and save again.
- If you edit the file while a change is pending in the settings window, the window can show "config changed externally; wait for the settings to refresh before saving". Wait a moment and try again.
- At startup, an invalid file does not stop the app. isoboard runs with the defaults and shows the error. It does not overwrite your file.

Unknown fields are errors, and unknown values are errors. Omitted fields use their defaults. To start over, see [reset isoboard settings](../../troubleshooting/reset-configuration/).

## version

| Field | Type | Default | Notes |
|---|---|---|---|
| `version` | integer | `1` | Must be `1`. Any other number gives "unsupported config version N; expected 1". A non-integer gives "config version must be an integer". |

## [capture]

```toml
[capture]
overlay_position = "top_right"

[[capture.bindings]]
id = "default"
shortcut = "CommandOrControl+Shift+2"

[[capture.bindings]]
id = "binding-1"
shortcut = "CommandOrControl+Shift+3"
profile_id = "profile-1"
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `overlay_position` | string | `"bottom_center"` | One of `top_left`, `top_center`, `top_right`, `middle_left`, `center`, `middle_right`, `bottom_left`, `bottom_center`, `bottom_right`. |
| `bindings` | array of tables | One default binding | Capture shortcuts. Must contain exactly one binding with `id = "default"`. |
| `bindings[].id` | string | None | Not empty and unique. The default binding uses `"default"`. isoboard names new ones `binding-1`, `binding-2` and so on. |
| `bindings[].shortcut` | string | None | A valid shortcut, unique across all capture shortcuts. See [how shortcuts are written](#how-are-shortcuts-written). |
| `bindings[].profile_id` | string | Omitted | Forbidden on the default binding, which uses the active Profile. Required on every other binding, and it must name an existing Profile ID. |

## [shortcuts]

| Field | Type | Default | Notes |
|---|---|---|---|
| `open` | string | `"CommandOrControl+Alt+KeyZ"` | The **Open isoboard window** shortcut. It must be valid and different from every capture shortcut. |

## [ui]

| Field | Type | Default | Notes |
|---|---|---|---|
| `mode` | string | `"basic"` | `basic`, `advanced` or `debug`, in lowercase. This is the footer **Display mode** switch. |
| `sounds_enabled` | boolean | `true` | The **Play sounds** switch. |

## [updates]

| Field | Type | Default | Notes |
|---|---|---|---|
| `check_automatically` | boolean | `true` | The **Check for updates automatically** switch. When `false`, isoboard checks only when you choose **Check for updates**. |

## [history]

| Field | Type | Default | Notes |
|---|---|---|---|
| `keep_originals` | boolean | `true` | The **Keep original screenshots** switch. Keeps the full-resolution image of the 50 most recent captures. Setting it to `false` in the app also deletes the images already kept. |

## How are shortcuts written?

A shortcut is one or more modifiers followed by one key, joined with `+`. The in-app recorder writes this form.

| Part | Values |
|---|---|
| Modifiers | `CommandOrControl` (`⌘ Cmd` on a Mac), `Control` (`⌃ Control`), `Alt` (`⌥ Option`), `Shift` (`⇧ Shift`) |
| Letters | `KeyA` to `KeyZ` |
| Digits | `Digit0` to `Digit9`. The default capture shortcut writes the digit as `2`, and both forms work. |
| Other keys | `Space`, `Enter`, `Tab`, `Backspace`, `Delete`, `Escape`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `F1` to `F24`, `Numpad0` to `Numpad9`, and punctuation keys such as `Minus`, `Comma`, `Period`, `Slash`, `Semicolon`, `Quote`, `Backquote`, `BracketLeft`, `BracketRight` and `Backslash` |

Examples: `CommandOrControl+Shift+Digit2`, `Control+Alt+KeyP`.

Names are not case-sensitive. The modifier names also accept `Ctrl`, `Option`, `Cmd`, `Command` and `Super`. Single characters such as `A`, `2` and `[` work in place of `KeyA`, `Digit2` and `BracketLeft`. Put the modifiers first and use exactly one key.

An invalid shortcut gives `invalid capture shortcut "..."` or `invalid open shortcut "..."`. A repeated one gives `duplicate capture shortcut "..."`. A conflict with the open shortcut gives `open shortcut "..." conflicts with a capture shortcut`. See [fix keyboard shortcut problems](../../troubleshooting/shortcuts/).

## [processing]

| Field | Type | Default | Notes |
|---|---|---|---|
| `active_profile` | string | `"default"` | The ID of the active Profile. It must exist. |
| `profiles` | table of tables | One Profile with ID `default` | Each key is a Profile ID. At least one Profile must exist. |

If you give `profiles` but omit `active_profile`, isoboard uses the first Profile ID in alphabetical order.

### [processing.profiles.\<id\>]

```toml
[processing.profiles.profile-1]
name = "Translate to Spanish"
additional_rules = "Translate all text to Spanish."
additional_rules_separate_pass = true
rules_model_mode = "same"

[processing.profiles.profile-1.conversion]
tables = true
colors = false
images = true

[processing.profiles.profile-1.model_settings]
provider = "openai_api"
model = "your-vision-model-id"

[processing.profiles.profile-1.model_settings.options]
reasoning = "low"
request_timeout_seconds = 180
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `name` | string | None | Required. Up to 80 characters, and unique ignoring case. |
| `model_settings` | table | Omitted | The model for the capture step. See below. A Profile without one is valid but cannot capture. |
| `rules_model_settings` | table | Omitted | A remembered, separate model for the rules step. Used only when `rules_model_mode` is `independent`. |
| `rules_model_mode` | string | `"same"` | `same` or `independent`. |
| `conversion.tables` | boolean | `true` | The **Tables** option. |
| `conversion.colors` | boolean | `true` | The **Colors** option. |
| `conversion.images` | boolean | `true` | The **Images** option. |
| `additional_rules` | string | `""` | The text of your Additional Rules. |
| `additional_rules_separate_pass` | boolean | `false` | Applies the rules in a second step. |
| `application_mode_override` | string | Omitted | `prompt`, `html_cleanup` or `prompt_and_html_cleanup`. Omitted means Automatic. Shown in Advanced mode. |
| `conversion_prompt_template` | string | Omitted | Advanced. A complete replacement for the bundled conversion prompt, as a Jinja template. Omitted uses the bundled prompt. |
| `postprocessing_prompt_template` | string | Omitted | Advanced. A complete replacement for the bundled rules-step prompt. |

Prompt templates are not appended to the bundled prompt. A template cannot be empty and must be valid Jinja. Replace a template only if you know what the prompt must keep. Use **Reset to default** in the **Profiles** tab to undo it. See [Profiles](../../use/profiles/) and [Additional Rules](../../use/additional-rules/).

Profile IDs are keys such as `default` or `profile-1`. They must not be empty. The Profile name is what you see in the app.

### model_settings and rules_model_settings

| Field | Type | Default | Notes |
|---|---|---|---|
| `provider` | string | None | Required. `local`, `openai_api`, `anthropic_api`, `gemini_api` or `openai_compatible`. |
| `model` | string | None | Required. Not empty. Copy the ID from the model picker. |
| `options` | table | Omitted | Optional tuning. |

For `provider = "local"`, `model` is the ID of a built-in local model. Copy it from a Profile that already uses the model.

### options

| Field | Type | Range | Notes |
|---|---|---|---|
| `temperature` | number | 0.0 to 2.0 | Advanced mode. Omitted uses the provider default. Local models accept only 0.0. |
| `max_output_tokens` | integer | 1 and up | Advanced mode. |
| `reasoning` | string | See below | Available in every mode. |
| `request_timeout_seconds` | integer | 1 to 3600 | Advanced mode. Omitted uses 120 seconds. |

`reasoning` accepts `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, `on` or `off`. What a provider supports varies by model. An explicit value that the provider cannot honor makes the capture fail with "the inference option reasoning is not supported by this provider or model". The `on` and `off` values are not supported at this time. Omitting the field is not the same as `none`. Unknown option names are errors.

## [inference]

```toml
[inference]
provisioned_providers = ["openai_api"]

[inference.providers.openai_api]
credential_ref = "0123456789abcdef0123456789abcdef"

[inference.providers.openai_compatible]
endpoint = "http://localhost:1234/v1"
credential_ref = "fedcba9876543210fedcba9876543210"
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `provisioned_providers` | array of strings | `[]` | Providers whose starting Profiles isoboard already created. It stops isoboard from recreating Profiles you deleted. Remove an entry to let isoboard create the starting Profiles again for a connected provider. |
| `providers.<id>` | table | None | One table per provider. The key must be `local`, `openai_api`, `anthropic_api`, `gemini_api` or `openai_compatible`. |
| `providers.<id>.endpoint` | string | Omitted | Only `openai_compatible` accepts it, and that provider requires it. |
| `providers.<id>.credential_ref` | string | Omitted | An opaque reference to the key in the macOS Keychain. isoboard creates and manages it. |

Endpoint rules: the URL must use `https://`, except `http://` is allowed for `localhost`, `127.0.0.1` and `[::1]`. A violation gives "inference endpoint must use HTTPS (HTTP is allowed only for localhost)". A missing endpoint on the compatible provider gives "openai_compatible providers require an endpoint".

Do not write or edit `credential_ref` by hand. An invalid value is rejected. If you remove it, the provider shows as not connected and you enter the key again on the **Providers** tab. The `local` provider accepts no credentials.

## Which errors can the file produce?

| Error | Cause |
|---|---|
| `could not parse config <path>: ...` | Invalid TOML, an unknown field, a wrong type or an unknown enum value. |
| `unsupported config version N; expected 1` | `version` is not `1`. |
| `processing must contain at least one profile` | The `profiles` table is present but empty. |
| `active profile "..." does not exist` | `active_profile` names a missing ID. |
| `duplicate profile name "..."` | Two Profiles share a name, ignoring case. |
| `profile name must be at most 80 characters` | The name is too long. |
| `additional rules exceed 65536 bytes` | `additional_rules` is too long. |
| `capture binding "..." targets missing profile "..."` | A binding points to a Profile that does not exist. |
| `capture.bindings must contain exactly one default binding` | The default binding is missing or repeated. |
| `unsupported provider "..."` | A key under `[inference.providers]` is not a supported provider ID. |
| `inference options temperature must be finite and between 0 and 2` | `temperature` is out of range. |
| `inference options request_timeout_seconds must be between 1 and 3600` | `request_timeout_seconds` is out of range. |

## Frequently asked questions

**Can I edit config.toml while isoboard is running?**

Yes. isoboard reloads the file shortly after you save it. An invalid edit is rejected and the last valid configuration stays active. Changes you make in the settings window rewrite the file without your comments.

**Where does isoboard store my API key?**

In the macOS Keychain, never in `config.toml`. The file holds only an opaque reference. See [what leaves your Mac](../../privacy/data/).

**How do I reset the configuration file?**

Quit isoboard, move `config.toml` out of its folder and open the app again. See [reset isoboard settings](../../troubleshooting/reset-configuration/).

## Related

- [Settings reference](../settings/)
- [Keyboard shortcuts](../keyboard-shortcuts/)
- [Reset isoboard settings](../../troubleshooting/reset-configuration/)
- [Profiles](../../use/profiles/)
