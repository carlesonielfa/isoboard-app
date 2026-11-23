---
title: Product specification
description: 'The isoboard product specification: intended behavior for setup, capture, Profiles, history, licensing and architecture. User docs describe the shipped product.'
sidebar:
  label: Product spec
  order: 14
---

This page is the product source of truth. It defines the intended behavior of
isoboard, and the user docs describe the shipped product. Where the two differ, the
code and the running app decide what has shipped, and this page records what is
intended. Implementation details live in the other developer pages, starting with
[Architecture](../architecture/).

isoboard turns screenshots into content ready to paste into other
apps. The backend can also convert image files, but no user-facing entry point
exposes that yet, so it is not a shipped feature. **isOCR™** is its custom fine-tuned OCR model. Display the name with the
raised ™ symbol in UI copy. Model labels such as isOCR1 0.8B omit it, and technical identifiers stay unchanged.

Settled product behavior belongs on this page. For how to build and run the app, see
[Setup](../setup/) and [Architecture](../architecture/).

## Platform and app behavior

Settings → About exposes third-party notices and MPL source-access
instructions from packaged resources in the system’s default text viewer.
Exact-version MPL source archives are
available offline with the app. Access does not require a license or account.

isoboard updates itself from signed releases. A release build checks the
update manifest one minute after launch and every six hours while Settings →
About → Check for updates automatically is on. The switch persists as
`[updates].check_automatically` in `config.toml` and is on by default. Check
for updates runs a check at any time. Development builds check only on request.
Only a newer release in the running major version is offered, because a
commercial license covers one major series. A found update downloads in the
background and its signature is verified against the public key in the app.
The menu bar and About then offer Restart to update. About → What's new lists
release notes newest first. The installed version is tagged Current and the
newest version found by the last check is tagged Latest. Notes for the
installed and older versions are bundled; newer ones come from the update
manifest with the check, before the update downloads. isoboard never restarts on
its own and does not install while a capture is in progress. A failed automatic
check is silent; a failed manual check shows its error in About.

The first release targets macOS 14 or later and runs as a menu bar utility. Keep
as much code cross-platform as possible for later Windows and Linux releases.
Isolate native capture, permissions, shortcuts, and clipboard integration behind
platform interfaces.
The shared clipboard path is not runtime-validated on Windows or Linux yet.
Pure Wayland support remains deferred.

The menu bar provides capture, one-shot capture with a chosen Profile, history, settings, and processing status. A one-shot Profile capture does not change the active Profile.
On macOS, menu actions and history entries use consistent monochrome Lucide
icons and the branded capture icon. Keep them at 16 points with native template
tinting for the menu appearance, selection, and disabled states. The processing
status icon reflects the current capture state.
Open isoboard keeps the same label and window icon in every capture state.
Its Capture region action uses the active Profile and displays the
configured default capture shortcut as a native menu accelerator. Keep it and
the Open isoboard accelerator in sync with configuration changes. The action is
disabled while setup is incomplete or a capture is in progress. Capture
shortcuts also do not start region selection until setup is complete. The
window activation shortcut remains available during setup.

The main window is a settings surface with a Capture split button right of the
navigation, aligned to the right edge of the header. Its visible text is the
active Profile name. The primary action starts region selection
using that profile. Its dropdown selects the same persisted active profile used by the menu
bar, without starting capture. The active profile is indicated in the menu.
The primary action is disabled until capture is ready. Both halves are disabled
while configuration is pending or capture is busy. Window-initiated capture hides
the window before native region selection. Rejecting the start restores the window
unless another capture is already in progress. After completion or cancellation,
the window stays hidden until reopened. Copy retains its existing
menu-bar and History locations.

The settings window opens at 800 x 600. It appears automatically on ordinary
launches on first execution, after the app version changes, and when the active
profile needs initial setup. Other launches start hidden. The last opened version
is stored separately from configuration, after the window is revealed successfully.
Login launches stay hidden and do not consume the first ordinary launch. It keeps native window controls and resizing, with a minimum
content size of 720 × 500 logical pixels. On macOS, an
overlay title bar hides the title text and background while retaining native
close, minimize, and zoom buttons. The header background is draggable. Clear
space is reserved beside the buttons for the Sora isoboard wordmark. Its visible
letters align vertically with the measured native button center. The buttons,
wordmark, centered navigation, and right-aligned Capture action share one header
row at every supported window size. Onboarding retains the
wordmark header without navigation or capture actions. Other platforms retain standard title bars. Left-clicking
the menu bar icon reveals and focuses it. Closing the window hides it without quitting. Reopening
keeps the current tab and state. Losing focus does not hide the window. The
dedicated global activation shortcut reveals and focuses it and is separate
from capture bindings.

The shell centers the horizontal settings tabs in a compact header, in this
order: History, Shortcuts, Profiles, Providers, and Settings. Inactive tabs show icons only. The active tab shows its icon
and label. The document does not scroll by default. Long sections scroll inside
their active tab. History keeps its list and detail regions independently usable
within that space. The history list and selected result stay side by side at
every supported window size. The footer places capture status on the left and
the display mode control on the right. Status uses a green ready dot, a green spinner while
selecting or processing, a yellow loading indicator for local model
loading, or a red error dot after a capture failure. Its short label has a hover
and focus detail. Settings contains the configuration file location and app version.

After the licensing choice, when the configuration loads with an incomplete active profile,
the settings surface begins with a Screen Recording permission step. It explains
the purpose of access before asking macOS, shows the current permission state,
opens the Screen Recording pane for recovery, and automatically refreshes when
the native window regains focus. Permission requests use the native macOS
authorization API without capturing screen content. The app compares its current access with a fresh,
non-prompting process check. Restart isoboard appears only when macOS has saved a
grant that is not effective in the running process. There is no manual check button.
The permission flow remembers a one-time window return before requesting access
or opening System Settings. Both Restart isoboard and macOS Quit & Reopen reveal
the window on the next launch, even when a model is already configured.
Continue is enabled only when access is available. A granted state is visually prominent. The same screen
offers an Enable notifications action that opens the native macOS authorization
prompt when permission has not been decided. Notifications are only for model
downloads. Declining never blocks
Continue. Returning from System Settings refreshes notification permission. The next step
presents a focused Download isOCR card. The progress step is labeled Set up AI Model
for both local and external providers. isOCR is the recommended local path and uses the bundled
isOCR1 0.8B catalog entry. Its download
size comes from the shared local-model state. Download saves local model settings
with default options directly in the active Profile without changing its other
settings. The card shares the normal download
service and shows progress, verification, Cancel, Resume and Retry. It waits for
explicit Continue after the complete bundle is verified and prepared. Setup
prepares isOCR automatically with a tiny synthetic image request through the
shared local worker. This loads its weights and exercises vision, prefill, and
decode without creating a capture, clipboard result, or history entry. Continue
waits for preparation. Failures remain on the card and Continue retries. The
normal five-minute idle sleep policy still applies. An existing download
shows Use isOCR. Interrupted downloads reopen setup on ordinary launch, with
partial files retained. Setup requires downloaded bundles for the active pipeline.
Normal capture readiness can still download catalog models on demand.

The secondary action, I'll use an external LLM provider, opens the existing
Providers pane. Back returns to the local-model card. A ready local backend
does not complete external-provider connection. Continue prefers a usable Efficient
profile for the provider just connected, regardless of other connected providers.
It falls back to another usable profile for that same provider, preserving user
edits and never recreating deleted profiles. Without a newly connected provider,
setup preserves a ready external selection or selects a usable connected-provider
profile. Local setup fills the active profile with isOCR, renames the untouched
Default profile to "isOCR - default" (with a number suffix on a name collision),
and skips model editing when its configuration is ready. Only missing or incomplete defaults require
manual configuration, with the specific readiness issue shown in the guide.
The manual Model step uses the canonical Profile editor. Continue requires the
active stages and their provider access to be ready. The final stage opens the Shortcuts tab and
shows the configured capture shortcut and points to the top-right Capture
button. That button glows while the final setup message is visible. The Settings pane offers an optional Open at
login switch. It is off
by default. A login launch starts in the menu bar with
the settings window hidden. An ordinary first launch still opens setup when
needed. The switch reads and writes the system login registration rather than
`config.toml`. Capture is also available in the header. Copy remains in its native menu bar surface.

Readiness is matched by provider backend and the exact current credential
reference. Stale asynchronous readiness for an older credential does not
complete a stage. An incomplete unused Profile does not trigger or
replace setup for an already configured active Profile. Set up later
becomes available after screen access and a provider are connected and hides
the guide for the current session. Settings exposes Restart initial setup below About so a user can
resume it, including after a partial setup.

## Licensing setup and evaluation

Before model or permission setup, an installation without an evaluation record
shows Continue without a license key and Activate commercial license. Continuing
starts one seven-day commercial evaluation (168 hours). A configured installation
shows this choice once and then returns to the configured app without repeating model setup.
Restart initial setup explicitly begins at the license screen, then continues
through permissions and model setup. It displays the existing trial or commercial
status, never restarts an evaluation, and never removes an activated license.
Personal, non-commercial use remains free after evaluation expiry. Work includes
employment, professional activities and organizational work, including nonprofits
and public institutions. Personal study and unpaid student coursework are personal use.
Code development and testing are free, including paid or employer-sponsored work.
That permission does not cover ordinary productive commercial use. Free distribution,
independent fork additions, branding and restricted resale/bundling/hosting follow
the bundled source license and commercial terms.

Rust owns evaluation dates in `licensing.json` beside, but separate from,
`config.toml`. Development uses the isolated development config directory.
Dates are UTC epoch seconds. Settings displays local dates and times. Setup
restarts and updates retain the original start. Removing app data can remove
this local record but does not grant another evaluation under the intended terms.
No account, payment card or network request is required for evaluation.

Settings shows one permission summary. A verified commercial license takes
precedence over evaluation status: Licensed for personal and commercial use.
The inline key input shows a key icon when empty and a primary submit arrow
when filled. Submitting validates through Polar's public-client API in Rust,
with no administrative token, device activation record or usage increment.
The organization and license-key benefit must match the configured isoboard
1.x entitlement. First activation requires a successful online check and secure
storage. A failed replacement preserves an existing license. Successful setup
activation continues without starting a trial or repeating completed model setup.

The key is stored in a dedicated managed Keychain namespace, isolated between
development and release. `commercial-license.json` holds an opaque credential
reference, masked key suffix, license identity, version coverage, validation
timestamps and status beside the config file. It contains no full key or customer
billing data. Persisted commercial state is independent from trial dates.
The production Polar environment is used in development too. Local storage remains
isolated. The benefit covers 1.x, including development package versions 0.x.
Other major versions must not inherit that entitlement automatically.

Background checks are attempted at most once per 24 hours, including failed
attempts, with the attempt timestamp persisted across launches. The scheduler
runs independently of the webview and starts no checks while capture is busy.
Manual retry is available after a verification error. Requests have bounded
connection/overall timeouts, refuse redirects, and do not log keys or responses.
Only recognized Polar rejections end cached commercial permission. Network,
rate-limit, authentication, malformed-response and service failures preserve it.
No offline expiry is imposed. A rotated or otherwise invalid key can be replaced
through the same input. No server or fingerprinting is involved.

For an active license, Settings shows the permission summary,
a Priority Email Support status line and an Email support button that composes a
message to support@isoboard.app in the default email app. Validation uses Polar’s Current API
without a version header. The license record's top-level `created_at` plus 12 calendar
months determines support expiry, clamping leap day to the last day of February.
The period begins at license issuance, regardless of purchase or device activation
time. Rotating a key on the same Polar record does not restart support. The cached
deadline works offline and is preserved if a refresh lacks a valid creation date.
First activation with missing or malformed creation data shows support status
unavailable. It never invalidates commercial rights. Support data refreshes only
with license validation, with no separate request or refresh control. Support
expiry is independent of perpetual commercial permission. Remove license clears
the local entitlement and its
Keychain entry without calling Polar or resetting trial dates. Failed secret
cleanup is retained for retry. It cannot restore the removed entitlement.

A Get a license button opens the configured Polar hosted checkout in the
default browser. Opening failures show a retryable inline error. Opening checkout
does not grant commercial entitlement. Company seat delivery and refund operations
are managed in Polar. Validation accepts keys for the configured benefit and does
not equate devices with seats.

After expiry, show one dismissible in-window notice on the next ordinary launch.
Login launches do not schedule the notice. It does not appear during capture or
interrupt processing. Dismissal survives restarts. Capture, history and settings
remain usable after expiry or a local licensing error. Corrupt dates are preserved
and shown as unavailable. Clock rollback never starts a fresh evaluation. Once
expiry has been recorded it remains expired. This is not tamper-resistant storage.

Setup and Settings retain links for offline license terms, commercial terms and
the privacy notice. The finalized `isoboard-terms-v2` package (effective 2026-11-13) is bundled at
those resource paths, with the supporting terms and Spanish copies alongside it.
The repository LICENSE.md and package metadata identify the custom source-available
license. Third-party licenses remain independent. Native packaged opening of the
new files remains a release verification step.

Public product settings live in `config/product.jsonc`, parsed by Rust and exported
as typed frontend values with the generated bindings. Trial copy and calculation share
`trialDays`. Support calculation uses `supportMonths`. The same file owns licensing
contact/link data, document resources, Polar settings and refresh intervals.
Changes require a rebuild and do not add a support-specific request or schedule.
These settings are not editable through customer `config.toml`.

Application validation limits, history retention defaults and feedback timing live
in `config/application.jsonc`. Rust validation and frontend editors share these
limits, including their error copy. Capture exit fading uses the same duration in
native presentation and CSS. Service settings live in `config/services.jsonc`.
Their listeners remain loopback-only. `config/model-defaults.jsonc` defines setup model selection,
provider labels and initial model choices. Provisioning still happens once per
provider and never rewrites user profiles. Provider capabilities remain defined
in Rust. The frontend fallback is exported with the generated bindings.

## Features

- Shortcut to capture screenshots and convert them to HTML for clipboard, including embedded images.
- Image file conversion is implemented in the backend but not yet exposed in the UI.
- Clipboard history where selecting an entry brings it back to the clipboard.
- Integration with both local models (finetuned models for the task) and external generic LLM providers.
- Enabling or disabling of specific parts of the conversion such as tables, colors, images, and other visual elements.
- Optional postprocessing for translation, color adjustment, or other transformations.
- TOML-backed configuration for easy editing and sharing of settings.
- An installation and configuration `SKILL.md` for agents to help users set up the released app.

## Capture and clipboard

Capture audio uses isolated synth effects from the promo: a short click and tone
when region selection starts, a latch when the image is ready, and a two-tone
confirmation after clipboard publication succeeds. Opening a file skips the
region-start cue. Cancellation adds no sound. The inference-error cue is unchanged.
The system region picker has no app-controlled drag audio.
Settings contains a Play sounds switch, enabled by default. It persists as
`[ui].sounds_enabled` in `config.toml` and gates all app cues, including errors.
Changes apply to subsequent cues without restarting. The Settings tab uses a cog icon.

- A global shortcut starts region selection. Escape and dismissed selections
  produce cancellation, without an error overlay, failure sound or failed
  History entry. Dismissing the backend's image file picker has the same behavior.
- Screen selections below a threshold in total area are cancelled before
  processing, without a processing overlay, image-ready sound, clipboard change,
  or History entry. The threshold uses captured image dimensions, so display
  scaling affects its on-screen size. The threshold is an area of 500 pixels
  (`MIN_REGION_CAPTURE_PIXELS` in `capture.rs`). Image file imports have no minimum area.
- The settings footer shows Setup incomplete while permission, the active
  profile, provider credentials, or shortcut registration is missing.
  It shows Ready only when these local capture prerequisites are available.
- Image files, when exposed, use the same processing flow as screenshots.
- Menu-bar capture, capture shortcuts, and file-picker flows leave the settings
  window in its current state. The header Capture action hides it before selection.
- Keep the menu bar icon monochrome, frame it with capture brackets while a region
  is being selected, animate it while processing, and report the
  latest status in the tooltip and status row. Open isoboard reveals the settings
  window for capture details. A floating overlay appears after region selection
  and shows processing, clipboard success, and failures. It uses the display
  where selection finishes and stays out of the region picker. Clicking anywhere
  opens its related pending, successful, or failed History row and dismisses that
  capture's overlay. Early failures without a History row open the History list.
  Its compact translucent surface has animated processing dots
  and fades before closing. All states keep the same compact size. Full failure
  details are available in History. Hovering pauses
  automatic dismissal. Leaving resumes the remaining time. Settings has a
  Capture status section. Show capture status turns the overlay on or off
  (`[capture].overlay_enabled`, on by default). Turning it off hides an
  overlay that is showing, and the menu bar, sounds, and History still report
  results. Position chooses one of nine presets with a dropdown containing a
  3×3 screen grid (`[capture].overlay_position`, `bottom_center` by default).
  Position is unavailable while the overlay is off.
  Success closes quickly and failure remains longer. The menu bar keeps the
  latest status.
- Explain missing capture permissions, unavailable models, and provider errors
  with a clear next step.
- Validation, inference, and preparation failures do not call the clipboard
  writer. Cancellation before publication also leaves the clipboard unchanged.
- HTML is the canonical conversion output.
- The renderer accepts the prompt's full document, body, or fragment, produces
  one canonical full HTML document, and derives plain text from its final body.
  The clipboard receives the sanitized body fragment and plain text as one item.
  Plain text keeps content and meaningful whitespace. Tables use spaces between
  cells and line breaks between rows, without generated borders or alignment
  padding. Formatting does not add Markdown or strikethrough marks.
- The exact allowed HTML subset, cleanup stages, and output limits are defined
  in [HTML output](../html-output/).
- RTF remains later representation work. After native publication starts, an
  operating-system failure may leave a partial clipboard write. Restoration is
  not guaranteed.
- Embedded images must remain usable after pasting, without depending on temporary
  source files.

## Clipboard history

Clipboard history is persisted in SQLite through a history interface and its
SQLite adapter. One table stores completed capture attempts with a success or
failure outcome. It is separate from TOML configuration and stores up to 1,000
successful clipboard publications, subject to a 500 MiB total storage cap. The
adapter enforces the count and byte limits transactionally while inserting and
pruning entries. A failed history write is reported separately after clipboard
publication succeeds. It does not turn a successful clipboard write into a
failed capture.

After image capture, a pending item appears in the settings History list with
a thumbnail and processing indicator. It stays in memory until the capture
settles. Clipboard publication turns it into a saved result with the same
identity. If processing or clipboard publication fails, it becomes a saved
failed attempt with the sanitized error code and message. The window allows
copying its timestamp, source, profile, error code, and message as
plain text for reports. Pending items have no clipboard or delete action.
Failed attempts keep a concise error message and optional bounded diagnostic
details. Copied error reports include these details. The History view displays
them only in Debug mode. Rendering diagnostics identify the failed preparation
check without depending on the original screenshot. Clipboard preparation and
publication failures retain the exact response from the stage that failed,
within the provider output size limit. Debug mode exposes the existing Copy raw
HTML action, which copies source text and never renders it. Raw failed output
counts toward the History byte limit. Failed attempts remain limited to 20.
Failed attempts have no result to restore. The newest 20 failed attempts are
retained with their own count limit, separate from the successful-result count
and byte cap. They store only a bounded thumbnail, capture metadata, and error
details, and the original screenshot when originals are kept. The menu bar lists successful results only.

Each successful entry stores the exact raw inference HTML, the canonical clean
full HTML, the sanitized body fragment, the derived plain-text fallback, a generated
thumbnail of the source image, the original image dimensions, the capture
timestamp, the source kind, and a snapshot of the profile ID and
display name. Successful and failed attempts also keep the selected provider
and model for each used pass, the conversion flags and effective application
mode, and whether Additional Rules or a separate pass were used. This snapshot
is frozen at capture start. It contains no credentials, endpoints, or rule text.
Older entries without a snapshot remain available. When Keep original
screenshots is on (the default), the original capture of the newest 50 entries
is kept unchanged, within 1 GiB and separately from the entry byte cap, so a
future reprocess can run on it. Turning the setting off deletes the kept
originals. The raw response and clean output keep the existing bounded
inference and rendering limits, and the
thumbnail is the only image kept for older entries. Stored thumbnails are
resized to at most 256 pixels on their longest edge and 256 KiB. History rows
count their text and thumbnail payloads against the 500 MiB cap.

The menu bar has a History submenu with the five most recent successful entries,
then a separator and View all history. Each entry uses its local capture time,
with a date when captured before today, and a short text preview. Selecting
one restores its clipboard result. View all history opens the History tab,
including failed attempts. Native menus do not provide a rich content surface. Settings uses a
scrollable vertical tab list that loads older entries as it scrolls. A search
field above it matches word prefixes in the entry text and Profile name,
ignoring case and accents, and highlights matches in each row. A filter button
inside the search field opens a floating menu that narrows by outcome, Profile,
content (tables, images, styling, code), and a range of days chosen on a
calendar, with Clear filters. Content is detected when a result is saved, so
results saved before this existed and failed attempts never match a content
filter. Result details list the detected content. Filtering keeps newest-first order and hides the pending capture.
Opening a specific entry from the menu bar or overlay clears the filters. Successful tabs contain the source thumbnail,
the start of the plain-text result, and a split copy button. Its left side
restores the saved rich clipboard content. Its chevron opens a menu with Copy
as plain text. Successful details show Tables, Colors, and Images switches in
**Options**. Their initial values are the saved capture settings. Changes update
the existing preview and both copy actions using deterministic HTML cleanup.
No inference runs. Switching away from an entry and back resets the selections.
Neither the saved result nor its capture-time settings change.
For Prompt only and Prompt + cleanup, originally disabled options cannot
be enabled. Originally enabled options can be disabled and restored. Changing a
Prompt-only switch explicitly requests HTML cleanup for that variation. Once a
switch has changed, cleanup remains active until the entry is reopened, including
when all switch values return to their initial settings.
Cleanup-only results with a retained baseline allow all options to toggle.
Restoration depends on what the model produced. Older entries without a baseline
support removal from the saved output only. Empty variations disable copying
and preserve the clipboard. Pending tabs show
processing. Failed tabs show the saved error. Selecting a successful tab opens
its rendered clean HTML beside the list. Both copy actions derive the full plain
text from the saved sanitized body, or the current variation when formatting
selections change. This also applies the current plain-text rules to older
entries without changing their stored data. The plain-text
option is disabled when the current text is empty or whitespace only. Copying either form does not
run inference. Successful and failed details show the
saved settings summary above the timestamp. Older entries say that settings are
unavailable. The clean preview scales down to fit its frame without cropping or
scrollbars while retaining the
available width. It does not upscale short content. Unstyled preview content
uses a sans-serif font. Explicit clean HTML styles take precedence. Tables
without an explicit width fill the preview. One-entry deletion is available from the same
surface. Delete is in each saved entry’s copy menu.
The preview/detail divider can be dragged or resized with the keyboard. Details
default to four rows, with a one-row minimum and scrolling for the remaining
metadata. Basic mode hides Additional Rules when none were applied.

Advanced shows the conversion mode and original image size. Debug includes
Advanced settings and shows raw HTML and clean HTML copy actions. Each HTML
action copies its full stored source to the plain-text clipboard for debugging.
The clean rich preview is capped at 32 KiB. Raw inference HTML is never rendered
as markup. Clean rich previews render only sanitized HTML in an isolated preview
surface. The former
Preview and HTML Result sections are removed because history is the source for
those views.

History contains captured user content and may include sensitive text or image
thumbnails. It is local app data and follows the app's storage permissions.
Users can delete one entry at a time. Deletion is logical from the history API's
point of view, while SQLite and filesystem recovery behavior is not a
secure-erasure guarantee.

## Considerations

### Display modes

isoboard should cater to both casual users and power users. Advanced reveals
more complex features:

- Custom LLM providers via OpenAI-compatible API endpoints.
- Inference options such as temperature, max output tokens, reasoning effort,
  request timeout, and prompt template overrides.
- Custom local model paths and adapters.

Everyday capture, profile selection, and common conversion settings should work
without opening advanced settings. Hiding advanced settings does not reset them.
Debug includes Advanced and exposes saved source and model outputs in History.

### Configuration and privacy

TOML stores settings and reusable profiles.
The TOML must be the source of truth.
The settings UI and edited TOML must describe the same configuration.
Editting a setting via UI should update the TOML, and editing the TOML should update the UI.
We might provide a schema for the TOML to validate it and provide autocompletion in editors.
Invalid values or missing profile references must produce a clear error without overwriting the last valid configuration.

The version 1 configuration stores capture shortcuts as a strict array under
`[capture].bindings`. It always contains one `default` binding. The default
binding has no `profile_id` and resolves to the active profile when pressed.
Additional bindings target one fixed profile. A binding is resolved
by its native shortcut ID while holding the configuration lock, then its
profile and binding IDs are copied into the capture context. Capture work does
not hold the configuration lock. Direct Capture actions resolve
the active profile when invoked.

The target profile of a fixed binding can be changed without re-recording its
shortcut. The default binding always follows the active profile and cannot be
retargeted. Retargeting validates the profile and persists it as one
configuration transaction. The native shortcut remains registered.
The Shortcuts tab lists Capture region first, then optional fixed-profile
capture shortcuts. The separate Open isoboard window shortcut follows these
capture actions.

This is the fully written version 1 shape. Empty or partial files are also
valid: an empty file, a file containing only `version = 1`, and omitted
sections or fields load the defaults shown here.

```toml
version = 1

[capture]
bindings = [{ id = "default", shortcut = "CommandOrControl+Shift+2" }]
overlay_enabled = true
overlay_position = "bottom_center"

[shortcuts]
open = "CommandOrControl+Alt+KeyZ"

[processing]
active_profile = "default"

[processing.profiles.default]
name = "Default"

[inference.providers]

```

Shortcut editing records the next key chord pressed by the user instead of
requiring an accelerator string to be typed. Recording modifier-only and repeat
events does not commit a chord. Escape or losing focus cancels the recording
and preserves the current values. All native capture bindings are suspended
for the recording lifetime and restored on cancel, focus loss, or after a
recorded chord. The activation shortcut is also suspended while recording so
all registered global shortcuts have the same lifecycle. Invalid or conflicting
edits keep the last valid registrations active. Revision checks and rollback
rules apply to every shortcut edit. The activation shortcut is stored under
`[shortcuts].open`. Its default `CommandOrControl+Alt+KeyZ` is layout-stable and
can be pressed with one hand. The ISO-only `IntlBackslash` code is rejected by
the recorder because the native shortcut libraries do not recognize it.

On macOS, the packaged app configuration file is
`~/Library/Application Support/app.isoboard/config.toml`. Debug
development runs may use a repository-local configuration path, but this must
not change the packaged app's configuration location or silently migrate an
existing file.

Version 1 stores profiles under `[processing.profiles.<id>]` and the active ID
under `[processing].active_profile`. Names are trimmed, unique ignoring case,
and limited to 80 characters. IDs remain stable when names change. At least one
profile must remain. Deleting the active profile selects the first remaining ID.
Duplicating copies every saved setting, selects the copy, and opens its editor.
Invalid edits preserve the last valid configuration.

Profiles own their primary `model_settings` and optional remembered
`rules_model_settings`. Each model configuration contains a fixed provider, model,
and optional inference options. `rules_model_mode` is `same` by default or
`independent`. Same mode reuses the complete primary configuration. Independent
mode requires its own configured model and never falls back when it is missing.

TOML is the source of truth. Switching to same mode retains one independent
configuration for later reuse, including across restarts. This is not a per-model
settings history. Inactive settings are validated structurally but do not affect
capture readiness or credential access. Profile duplication copies both
configurations independently. Credentials and downloaded model resources remain
shared.

For an OCR-only primary model, the editor always shows Apply rules in a separate
pass checked and disabled, explaining that a text-capable model must apply custom
rules. With empty rules this is UI-only: no second pass runs, and its model editor
stays hidden. Entering rules reveals the editor. Clearing them hides it while
preserving remembered settings.

With meaningful rules, an OCR-only primary model automatically enables a separate
pass and independent rules-model selection. The editor guides the user to choose
a text-capable model. It never silently chooses an external provider. Empty rules
skip the second pass. Both active stages and all processing settings are frozen
at capture start.

Connecting a supported provider creates its Efficient and Accurate starting
Profiles once. These are ordinary editable and deletable profiles, with concrete
model IDs stored in TOML. `[inference].provisioned_providers` records which
providers have been seeded, preventing startup, reload, reconnect, or readiness
refresh from overwriting edits or recreating deleted profiles. Removing a provider
credential does not delete its profiles. OpenAI-compatible providers have no
bundled model choices and use manual model configuration.

Prompt template fields are exposed only in Advanced settings. The conversion
template field is always shown there. The postprocessing template field is shown
only when separate-pass processing is enabled. Both fields are initially
prepopulated with their bundled defaults. The profile editor saves a template as
`null` when its source is byte-for-byte equal to that default, so the bundled
template remains authoritative by default. Any whitespace change is a real
override. An edited empty template is invalid and leaves the last valid profile
unchanged. Non-null sources are syntactically validated as Jinja when saved and
rendered with typed, frozen run context. They replace the corresponding bundled
template rather than being appended to it. Additional Rules remain literal data
and are never evaluated as Jinja source. The snapshot exposes the bundled
conversion and postprocessing defaults once as `promptTemplateDefaults` for UI
prefill and exact comparison.

Additional Rules are limited to 64 KiB of UTF-8 bytes before whitespace
normalization. Control characters other than LF, CR, and TAB are rejected.
Optional model configurations may remain absent during setup. Explicit invalid
models or options are rejected. Provider availability is checked for active stages.
Provider records remain under `[inference.providers]`, keyed by the fixed backend
enum, and contain only optional endpoint and managed credential references.
Managed credentials never appear as raw values in TOML or frontend snapshots.

The version-one inference options are optional and live in a nested `options`
table on each owned model configuration. It may contain `temperature` (a finite number
from 0.0 through 2.0), `max_output_tokens` (a positive integer),
`request_timeout_seconds` (1 through 3600 seconds), and `reasoning` (one
of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`, `on`, or `off`).
An omitted option is not serialized and lets the provider or runtime default
apply. Explicit `reasoning = "none"` remains distinct from an omitted value.
If every option is omitted, the `options` table is omitted too. The runtime
validates and persists these values, freezes them with each resolved stage
profile, and applies them to image and text requests on API routes. An unset timeout uses the 120-second runtime default. Explicit stage
profiles may have independent frozen timeouts, and the timeout covers the whole
response stream. Temperature and max-output values are omitted from the wire
request when unset. Reasoning is provider/model-specific: supported values are
mapped by the adapter, while unsupported explicit options fail with an
actionable provider error. The generic adapter does not implement the `on`/`off`
toggle values, so those explicit values fail rather than being silently
coerced, ignored, or approximated. Model IDs with reasoning suffixes retain
genai's provider-specific interpretation. Unknown catalog metadata must not
cause stored values to be discarded.

For example, options for the primary model are stored as:

```toml
[processing.profiles.default.model_settings.options]
temperature = 0.2
max_output_tokens = 4096
reasoning = "none"
request_timeout_seconds = 120
```

The options table is omitted when all four values are unset. These values are
validated configuration and catalog/UI metadata, carried by the frozen stage
profile, and applied by the provider adapter. The profile editor exposes
reasoning in Basic mode and temperature, max output, and timeout in Advanced
mode. It autosaves the complete draft when an edit is committed, sends `null`
when every option is unset, and keeps invalid numeric edits local with an
inline error.

The model configuration editor places the model picker and reasoning selector in
one responsive row. Reasoning remains visible for every model: it is disabled
when models.dev reports no supported control, only a toggle, or only a budget
control, because the current
runtime does not transport those controls. When both toggle and effort are
declared, effort choices take precedence. Unknown and manually entered models
keep it enabled. A saved
reasoning value is retained when the control becomes unavailable. Changing the
model does not reset it.

Settings and capture availability checks never read Keychain. They use saved
references and session state. An unread saved credential is Configured, not
verified. External inference reads the credential on first use. Successful reads
are cached for the process in development and release builds. Replacement and
token refresh update the cache. Deletion clears the secret. Access errors are
remembered for status display without background retries. An explicit capture
can retry denied access. Local inference does not access Keychain.
It emits HTML only after a completed provider response. Capture then normalizes
the prompt's HTML, embeds validated image crops, writes the sanitized body
fragment and derived plain-text fallback through the clipboard interface, and
emits `completed` with the canonical full document only after that write
succeeds. Clipboard history follows the settled SQLite contract above. RTF
remains downstream work.

This pre-release configuration accepts an empty file, `version = 1` alone, or
any omitted section or field and fills in the defaults. Explicit invalid or
unknown values, unknown fields, and unsupported versions are rejected without
overwriting the last valid configuration. Clearing the file is a valid reset.
No migration or manual rewrite is required. This checkpoint includes the
inference settings surface and capture-time HTML inference. A processing
profile without an inference assignment reports a sanitized failure.

The global settings footer mode is persisted in the optional top-level `ui`
table. `mode` is `basic` by default and may be set to `advanced` or `debug`.
Debug includes all Advanced settings and shows source-output copy actions in
History. An omitted table or field loads Basic without rewriting the file.

```toml
[ui]
mode = "advanced"
```

Keep credentials out of shared configuration. See [Secrets and Keychain](../secrets/)
for the storage boundary and [Inference and prompts](../inference/) for the
provider-neutral settings.

### Processing-profile contract

```toml
[processing]
active_profile = "default"

[processing.profiles.default]
name = "Default"
additional_rules = ""
additional_rules_separate_pass = false
rules_model_mode = "same"

[processing.profiles.default.conversion]
tables = true
colors = true
images = true

[processing.profiles.default.model_settings]
provider = "local"
model = "isocr1-0.8b"

# Optional independent rules configuration, remembered even in same mode.
[processing.profiles.default.rules_model_settings]
provider = "local"
model = "qwen3.5-2b"

[inference]
provisioned_providers = []

[inference.providers]
```

The initial conversion checkboxes cover tables, colors, and images, and are all
enabled by default. A compact illustrative skeleton at the right edge responds to
each switch without model inference. It shows table cells flattening into lines
while retaining colored header text when colors remain enabled. Disabling colors
neutralizes the header and paragraph accents. The image region disappears while
its caption remains. One shared Advanced-only
**Conversion settings application**
select applies to those checkboxes. Its options are **Automatic**, **Prompt
only**, **Cleanup only**, and **Prompt + cleanup**. The persisted
`application_mode_override` values are `prompt`, `html_cleanup`, and
`prompt_and_html_cleanup`. Omitting the field means Automatic. Automatic has an
effective label of **Automatic — Prompt + cleanup** for every model. For
isOCR, which uses a fixed extraction prompt, this has the same effect as cleanup alone. Prompt
only adds selected extraction clauses to the checked-in Jinja template at
`src-tauri/src/inference/conversion.jinja`. Cleanup
only leaves that prompt unchanged and applies only selected deterministic
final-HTML policies after the model call. Prompt + cleanup does both. An
explicit override survives model changes. The effective mode and all three
flags are frozen at run start. The mode does not apply to Additional Rules.

With all three checkboxes enabled and no Additional Rules, the default rendered
prompt is byte-for-byte identical to the current extraction prompt. The
template must retain the mandatory HTML safety and output-envelope requirements.
All trusted first-pass wording and prompt conditionals, including the Additional
Rules section, belong in that template. Rust supplies typed context and owns
rendering, caching, readiness, and transport, not instruction text. User rules
are literal context data, never parsed or evaluated as Jinja source. Future stage
prompts must also use authoritative bundled Jinja templates.
Image-marker syntax and validation apply when image extraction is enabled or
markers are emitted. Feature-preservation clauses are conditional on the
selected checkboxes. They must not weaken the safety invariants. Additional
Rules may request content transformations in the future, but cannot bypass
those invariants.

Additional Rules is one multiline field. The trusted conversion template includes
meaningful rules literally in its Additional Rules section when
`additional_rules_separate_pass` is false. Meaningful whitespace, template-like
syntax, HTML, and quotes remain unchanged. Whitespace-only rules use empty
context, omit that section, and preserve the base prompt byte-for-byte.
When the flag is true with meaningful rules, the resolved stage chains and known
credential references are preflighted before the first transport. The first image
call excludes rules. The second is text-only and receives literal rules plus
sanitized intermediate HTML rendered by
`postprocessing.jinja`. The postprocessing template applies only the Additional
Rules and preserves everything else. Feature cleanup remains deterministic app
logic after the final call. Empty rules use the ordinary one-call path.

The split intermediate is safety-sanitized HTML. Each valid crop marker is
validated once and allocated a short private run-local reference such as `i1` via
`data-isoboard-image-ref`. Physical crops are prepared once before request two,
even when `images` is disabled. The second stage receives no screenshots, crop
base64, coordinates, or original provider image attributes. Its raw HTML rejects
external/data sources, coordinates, missing/duplicate/unknown refs.
Known refs may be removed, reordered, or repeated, subject to the existing
per-crop, aggregate, document, and 32-final-occurrence budgets. Valid crops
covering less than 0.1% of the screenshot area are omitted without failing
conversion. This relative cutoff applies to direct and separate-pass conversion.
Malformed or reversed coordinates remain errors. Finalization
resolves only current-run references, applies cleanup after the last model call,
embeds crops, derives plain text, and publishes clipboard then history once.
Failure, cancellation, or invalid second output publishes neither.

For split runs, history stores the exact raw response from each stage without
rendering it. Normal history shows only the final result. Debug history
exposes each stage output for copying to the plain-text clipboard. A profile
may be valid but execution-unready due to missing credentials, unavailable
models, or an unassigned reference. Manual
model IDs and catalog availability are advisory for saving. Readiness is
reported at execution time.

The initial disabled-feature policies are settled. When tables are disabled,
rows become text blocks with spaces between cells and preserve content and formatting
inside each cell. Merged-cell
content is emitted once, captions are preserved, and reading order is retained.
When colors are disabled, color styling is removed while content and other
formatting remain. When images are disabled, image regions are omitted while
surrounding text and captions remain. Text printed inside an omitted image is
not transcribed as a substitute. The complete visual-feature inventory beyond
tables, colors, and images remains open. Plain-text profile behavior is deferred.

The profile editor embeds model settings directly. Both model editors use the
same component and save through the owning profile. Provider selection uses the
shared select over configured fixed backends, with Local available without
credentials. Missing access directs the user to Providers. Capture shortcut
selection uses SettingsSelect and retains the unavailable-profile option.

Show which provider and model the active profile uses. Local inference keeps
image processing on the device. External inference sends the required input to
the selected provider. Do not silently fall back from local to external inference.
Diagnostics should exclude credentials, captures, and extracted content by default.

model configuration model selection uses the public models.dev catalog as advisory
metadata. The picker searches friendly names and model IDs, accepts arbitrary
model IDs entered directly, and sorts catalog entries by newest release date
first. The two app-defined choices for each built-in provider appear above
catalog entries with lightning and bulb icons. Hovering each icon explains the
speed and capability tradeoff. The product UI uses a text-output filter that
includes vision LLMs even when image-input metadata is absent. Capability
metadata is advisory and does not hard-block a saved model or profile. The catalog
maps OpenAI API
profiles to `openai`, Anthropic API profiles to `anthropic`, and Gemini API
profiles to Google's `google`
provider. OpenAI-compatible profiles are manual-only. Suggestions do not establish
account entitlement. Provider readiness and the inference request remain
authoritative.

The native service loads a last-known-good catalog cache before refreshing in the
background. The cache is disposable application data: its internal format has no
compatibility or migration contract. A cache with an unsupported format is
discarded and fetched again. No legacy-format branches are retained. Refresh
uses conditional `ETag` and `Last-Modified` requests and a SHA-256 body hash,
retaining the existing entries when the body is unchanged. Network failure
leaves a cached catalog available but stale, and a missing or discarded cache
leaves the catalog unavailable until a refresh succeeds. Catalog availability
and capability metadata do not gate the opaque model ID stored in an Inference
Profile.

## Architecture

A **Profile** defines conversion settings, rules, and owned model configurations
for each stage. Provider authentication and local model resources remain shared.

The active Profile applies to both screenshots and opened images.
Plain-text profile behavior is deferred.

Users can set up multiple Profiles and switch between them.

Capture shortcuts are an extensible list of bindings. The list includes one
default binding that uses the Profile active when the shortcut is
invoked. Users can add bindings that target a specific Profile,
allowing direct access to that profile without changing the active profile.
Deleting a Profile also deletes every fixed-profile capture binding
that targets it. The default active-profile binding is preserved.

### Execution flow

Input comes from a screenshot shortcut. The backend also accepts opened image files. Capture freezes the
selected conversion flags, effective application mode, Additional Rules, both
stage assignments, prompt template sources, and separate-pass flag at run start.
With separate-pass mode off, the trusted
conversion template includes meaningful rules literally in the first
image-to-HTML prompt. Empty or whitespace-only rules omit the Additional Rules
section. With meaningful rules and separate-pass mode enabled, both stage
assignments and known credential references are preflighted before transport.
The first image call excludes rules, then a text-only second call uses the
frozen postprofile, literal rules, and sanitized intermediate HTML
from `postprocessing.jinja`. In same-model mode, the frozen first-stage settings are reused. Independent
mode never falls back when its model is missing or invalid.
Empty rules with the flag enabled remain an ordinary one-call conversion.

The flow below shows the split branch. Empty rules take the ordinary one-call
path.

1. **Input.** A screenshot arrives through the capture shortcut. Opened image files can also enter the pipeline in the backend, but no user-facing entry point exists yet.
2. **Profile selection.** The Profile decides whether Additional Rules run in a separate pass.
3. **Single pass.** When the rules are empty or the Profile uses one call, the model converts the content and applies the rules together.
4. **Split pass.** When rules need their own pass, the first call extracts content and structure. If rules are present, a second call applies them to that content. With no rules, the first result goes straight to the clipboard.
5. **Output.** The result is written to the clipboard as HTML and native representations.

### Profile configuration

Profiles own their primary `model_settings` and optional remembered
`rules_model_settings`. Each model configuration contains a fixed provider, model,
and optional inference options. `rules_model_mode` is `same` by default or
`independent`. Same mode reuses the complete primary configuration. Independent
mode requires its own configured model and never falls back when it is missing.

TOML is the source of truth. Switching to same mode retains one independent
configuration for later reuse, including across restarts. This is not a per-model
settings history. Inactive settings are validated structurally but do not affect
capture readiness or credential access. Profile duplication copies both
configurations independently. Credentials and downloaded model resources remain
shared.

With meaningful rules, an OCR-only primary model automatically enables a separate
pass and independent rules-model selection. The editor guides the user to choose
a text-capable model. It never silently chooses an external provider. Empty rules
skip the second pass. Both active stages and all processing settings are frozen
at capture start.

Connecting a supported provider creates its Efficient and Accurate starting
Profiles once. These are ordinary editable and deletable profiles, with concrete
model IDs stored in TOML. `[inference].provisioned_providers` records which
providers have been seeded, preventing startup, reload, reconnect, or readiness
refresh from overwriting edits or recreating deleted profiles. Removing a provider
credential does not delete its profiles. OpenAI-compatible providers have no
bundled model choices and use manual model configuration.

The intended backend and model options are:

| Backend family        | Integration                  | Provider / model options                                        |
| --------------------- | ---------------------------- | --------------------------------------------------------------- |
| Local                 | llama.cpp                    | Compatible general-purpose local models; isOCR                  |
| External API          | `genai` crate                | OpenAI-compatible server; OpenAI API; Anthropic API; Gemini API |

The Local provider is automatically available in builds with llama.cpp support
and requires no credentials. An model configuration chooses its model and owns
download readiness. Downloaded sleeping models remain ready and wake on
capture. One dedicated worker serializes requests, reuses the resident model,
releases it before switching, and unloads after five minutes idle. Local requests
use the existing capture, HTML cleanup, clipboard, History, and failure paths.
They never fall back to an external provider.

Debug mode shows native-process resident RAM and loaded local model count
beside the footer status. RAM is sampled every two seconds while the debug
readout is visible, and model count follows shared runtime state. Downloaded
but sleeping models are not counted as loaded. The RAM value includes native
app overhead. Separate WebView helpers and complete GPU-memory accounting are
outside this measurement.

Each local model declares a minimum UI mode for picker suggestions. Basic models
appear in all modes, Advanced models in Advanced and Debug, and Debug models
only in Debug. isOCR1 is visible in all modes. Qwen is
Debug-only. Saved selections and capture readiness are independent of this
presentation filter.

Local model definitions are bundled in `src-tauri/src/local_models/catalog.json`
and require no startup network request. The catalog contains isOCR1 0.8B and Qwen3.5-2B. isOCR uses
its fixed extraction prompt. Additional Rules require a separate text-capable
model configuration. HTML cleanup applies its conversion flags. Published curated models that are not downloaded download on first capture.
The overlay and footer show Downloading model with measured bundle progress,
then Loading model during a cold load or wake, then Processing image. Loading
has no percentage because the native loader does not report one. Downloaded
models skip the download stage. Warm models also skip loading. The menu status
uses the same stages. Files are pinned by revision, size, and SHA-256, and a
complete verified bundle is published atomically. Model controls include an info icon
that opens the model's Hugging Face page for model details and licensing. While a
model is not downloaded, Setup and the Profile editor show that downloading it
accepts its license, linking to the catalog's license file at the pinned revision
or, when the catalog names none, to the model page. The app does not download or validate
model license documents. Failures leave no ready partial
bundle and a later capture retries the download. isOCR uses the published
`carlesonielfa/isOCR1-0.8B-GGUF` repository. Model editors expose Download, Cancel, Resume/Retry, bundle size and measured
progress. Partial downloads survive cancellation, transport failure and app restart.
Resume validates the byte range and the complete pinned checksum before publishing
both model and projector together. While downloading, the UI explains that users
can leave the window with isoboard running. With notification access enabled, a
native notification announces successful verified completion from the shared
download service, even while the window is hidden. Cancellation, failure, and
already downloaded bundles do not produce completion notifications. Notification
delivery failure does not fail the download.
Remove download asks for confirmation, names the model, and explains that profiles
using it will need another download. Cancelling keeps the files and runtime intact.
Confirming deletes downloaded and partial assets. Removal is blocked until captures and model downloads finish, and unloads
the selected model before deleting its files. Runtime loading is communicated by capture state.
There is no manual unload control.
Model state and files are shared across model configurations. Cancelling a capture
stops that capture's wait. An explicit Cancel download stops the shared download. No user-facing arbitrary file-path configuration is added.
See [Local models](../local-models/) for the runtime, catalog, downloads, and probe.

The repository includes an opt-in `api-probe` Cargo feature for the external API
transport slice. It supports the closed provider set OpenAI, Anthropic, Gemini,
and OpenAI-compatible. The provider and model are explicit, and the compatible
provider requires an explicit endpoint. Development API keys are stored in
separate provider-specific macOS Keychain slots after being entered through the
CLI, and are not part of TOML or Profiles. The probe sends only a generated image
and is not a release default or product integration. It shares the bounded
inference runner with normal capture, which owns image requests, cancellation,
stream completion, limits, and sanitized errors. See
[Inference and prompts](../inference/#api-probe) for the commands.

A conversion or combined call needs an image-capable model. A separate
postprocessing call needs a model that supports the chosen rules and intermediate
content. Catalog capability metadata is advisory in Step 1: surface known
limitations, but do not block saving an opaque or manually entered model ID. The
provider adapter remains authoritative at execution time.

The isOCR distribution, model version, preprocessing, prompt, and supported local
formats must be defined before integration. Custom models and adapters must match
the selected backend. Prototype model paths and inference settings are not defaults.

The diagrams specify the workflow and profile relationships. Final conversion
output is HTML. The split intermediate and final assembly rules are defined in
the profile contract above.

## Tech stack

Tauri 2 + Rust, with Svelte 5, SvelteKit, TypeScript, and Vite. Bun manages frontend
dependencies and runs development commands.

Use native APIs where needed for window presentation and effects, window focus,
window dimming, screenshot capture, permissions, global shortcuts, and clipboard
access. Keep processing, profiles, and provider logic separate from
platform-specific code. The app uses dark appearance for its native windows,
backdrop materials, and frontend regardless of the system theme. Light
appearance is deferred until a complete light theme is implemented.
The main UI uses a prismatic plum theme with bundled Sora headings and Inter
body/control text. The dotted decoration is confined to the content region.
Settings navigation and the application footer remain dot-free. The capture
status overlay shares the prismatic surface and faded dot treatment, with Sora
status titles, Inter details, and compact status badges. Captured HTML remains isolated
from the application theme.
The desktop adapter uses a decorated, transparent
window with native Popover backdrop material on macOS. The frontend applies a
translucent shell tint and backdrop blur there, with an opaque fallback on
other platforms. Transparent macOS windows require a private API and prevent
App Store distribution. Distribution and signing remain open decisions.

## History formatting baseline

Successful captures retain an optional sanitized formatting baseline after the
last model pass and before feature cleanup, with generated image crops embedded.
History byte accounting includes the baseline under the existing 500 MiB limit.
Raw model responses remain separate
from this safe baseline and are never rendered.

## Profile navigation

Profiles uses vertical tabs beside the selected editor, matching History's
sidebar layout. Selecting a tab also selects that profile for captures. Add is
an icon-only plus below the last profile. Duplicate and Delete are in each row's
chevron menu. A filled marker identifies the active profile. The selected row uses
the same surface as History. Creating or
duplicating a profile selects it and focuses its name. The final profile cannot
be deleted. The sidebar and editor scroll independently. The editor shows Conversion before
Model, followed by Rules. Separate-pass model usage uses a full-width sliding
two-option control. The same-model option shows the current primary model name
on a separate line in monospace.

## Profile readiness presentation

Outside initial setup, the footer identifies profile blockers as Profile needs
attention. Screen access, restart requirements, and shortcut failures have their
own labels. Pending checks show Checking without marking the profile invalid.
Initial setup retains its existing entry and completion rules.

Profiles share one assessment for the saved sidebar state, draft editor notices,
and active capture readiness. Only stages used by the capture are assessed.
Manual remote model IDs remain valid without a catalog match. Provider readiness
must match the current credential reference. The editor keeps actionable blocking
notices and save failures at the top while scrolling. Recovery actions focus the
relevant settings or open Providers. Save failures describe unsaved edits.
The saved profile remains the source of capture readiness. Invalid profiles remain
selectable and show an error icon and short red reason instead of a model subtitle.
