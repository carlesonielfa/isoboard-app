---
title: UI system
description: The isoboard design tokens, window and overlay surfaces, shared component inventory, shadcn-svelte rules, and copy and legibility conventions.
sidebar:
  order: 11
---

isoboard's UI is a compact, quiet dark utility on a deep plum canvas, with a restrained pink, blue, and lavender wash, white text, lavender accents, and thin translucent borders. Every displayed element is a Svelte component in `src/lib/components`, built from Tailwind utilities and the semantic tokens in `src/app.css`. This page is the component inventory and the rules for extending it.

Inter is bundled for body copy and controls, Sora for headings and wordmarks, and technical values keep monospace. The app always uses a dark appearance, regardless of the system theme. For frontend commands see [Setup](../setup/) and [Testing and CI](../testing-and-ci/). For the generated IPC bindings that components consume, see [Architecture](../architecture/).

## Rules for adding UI

- Every displayed element is a component under `src/lib/components`. Pages compose components and do not define displayed elements directly.
- Before building a control, search this page, `src/lib/components`, and `src/lib/components/ui` for existing behavior and presentation. Reuse or extend before creating.
- Extract generic UI at its first use, even with one caller. If a control's layout or interaction can serve an unrelated feature (a split button, an action menu, a status badge), build it as a shared component. Pass content, callbacks, and state through props or snippets. Keep feature names and feature types out of its API.
- Feature components own domain data, labels, actions, and state. They compose shared controls and must not copy generic markup, keyboard behavior, or styling. Add a wrapper only when it owns meaningful domain behavior, not when it renames or forwards a shared control.
- When controls need different semantics, such as value selection versus an action menu, use the matching behavior primitive and share the surface and item presentation. Never create an independently styled dropdown.
- Document every new shared control on this page, with its intended use and how it composes with existing controls. Before finishing, review new components for generic markup left in feature code, and run `make frontend-check` and `make frontend-lint`.
- Prefer direct actions and autosave when the result is clear without a confirmation control. Keep layouts stable while an action runs.

## Tokens and styling

Tailwind utilities are the default for layout, spacing, typography, and states. `src/app.css` holds only the `@theme` identity tokens, document defaults, and small visual effects that belong to no component, such as the compact capture grid and spinner keyframes. The content grid belongs to `ContentSurface`.

Tailwind 4 runs through the official Vite plugin and is imported from `src/app.css`. A component `<style>` block that uses Tailwind features such as `@apply` needs `@reference '../app.css';` first, with the path relative to the component.

Use Tailwind's built-in spacing and type scales. Add a token only for product identity or a shared visual value, and when a shared value is missing, add one semantic token at the theme layer and reuse it.

| Token group | Examples |
|---|---|
| Surfaces | `canvas`, `surface`, `surface-shell`, `surface-solid`, `surface-raised`, `surface-control`, `surface-overlay`, `surface-preview` |
| Text and lines | `ink`, `border`, `border-strong`, `highlight`, `highlight-surface`, `focus`, `grid-dot` |
| Status | `danger`, `danger-border`, `danger-surface`, `warning`, `success` |
| Shape and effect | `rounded-control` (12 px), `rounded-surface` (20 px), `rounded-capture` (16 px), `shadow-glow`, `bg-prismatic`, `toolbar-button` height (30 px) |
| Fonts | `font-sans` (Inter Variable), `font-display` (Sora Variable) |

### Color rules

- Component class strings use semantic tokens. Do not use raw color literals (`#`, `rgb`, `rgba`) or palette colors such as `white`, `black`, or `red`. Use Tailwind opacity modifiers on tokens for tints.
- Errors share `text-danger`, `border-danger-border`, and `bg-danger-surface`. The top error toast uses the opaque surface token, and inline messages may use an opacity variant.
- Configured or completed states use `success`. A persisted provider credential may use it to show configuration. It does not claim that live authentication has been verified.
- The primary action is white with plum canvas text. Secondary actions use the outline button. Active navigation and the mode indicator use lavender fills with canvas-colored text. Focus rings are lavender and stay bright. Avoid decorative glow on active navigation.
- Controls and compact groups keep 12 px corners. Larger onboarding surfaces use 20 px.
- When a component has finite variants, map each variant to a complete static class string so Tailwind can discover it. Use static utility classes in new components.

### Text legibility

Normal UI text uses the standard foreground color, including metadata, help, configuration, and empty-state copy. Do not use muted, subtle, or reduced-opacity text for hierarchy. Reserve dimmed text for a meaningful interaction state such as disabled, hover, pressed, or invalid, and keep accessible contrast. Error and warning text use the semantic status colors.

The one exception is the Additional Rules example placeholder. It shows inside the empty field in muted italic text, and typed rules use the normal foreground color and type style.

### UI copy

- Keep displayed copy minimal, with labels and actions first.
- Add help text only when it is needed to finish an action or recover from an error.
- Remove redundant status messages and implementation details from the normal settings surface.
- Keep actionable errors and useful configuration access available.
- Expose the next action when an error needs recovery, and preserve keyboard focus, disabled states, and live-region behavior.

## Window and surfaces

<!-- os:macos -->
On macOS the main window uses `titleBarStyle: Overlay` and `hiddenTitle`, keeping native traffic-light controls inset at (14, 29) logical pixels. The Sora isoboard wordmark sits immediately to their right, and its visible glyphs are centered on the measured native button center in webview coordinates. Alignment refreshes when the header resizes or the window gains focus. Window content uses the native Popover material under a translucent `surface-shell` tint and `backdrop-blur-md`. Other platforms keep native title bars and use the opaque `canvas` fallback. Keep `surface` for grouped app surfaces.

- The header holds the controls, wordmark, centered navigation, and the right-aligned capture action on one row at every supported size, with matching 8 px top and right insets. Onboarding keeps the wordmark header without navigation or capture actions.
- The header and footer have no tint, backdrop filter, or dots. `AppShell` paints one continuous prismatic wash over the window material, so separate glass surfaces never composite over different parts of the window. The shell owns the solid reduced-transparency fallback.
- The dot pattern belongs only to `ContentSurface`: a static 28 px lavender grid at 30% peak opacity with a soft elliptical fade. Menus, dialogs, and status popovers are dot-free.
- The settings window has a minimum content size of 720 by 500 logical pixels. Top-level settings panes are flat, because the tab label supplies the section context. Use borders, backgrounds, and grouped surfaces only for meaningful inner structures such as rows, editors, and the History master and detail view.
- History fills its tab edge to edge with an 18 rem list beside the detail column at every width, and each column scrolls on its own. Profiles also fill their pane, with a 14 rem sidebar and a padded editor. Other panes keep the shared readable inset.
- Reduced transparency uses a solid plum shell and solid menu and capture surfaces.
- At the final setup step a soft lavender glow pulses around the Capture button until setup is dismissed or a capture begins. Reduced-motion settings keep it static.

### Capture overlay

The overlay shares the prismatic shell and the faded dot pattern, with 16 px outer corners, Sora status titles, Inter details, and the same glass treatment and opaque fallback as the main window. Every state uses one 288 by 60 px panel with a centered 32 px badge (12 px corners), lavender for processing and semantic success or error colors with Lucide icons, each with a 15% tint behind it. Static messages fit in full and variable details truncate. Full failure details live in History.

- One transparent button covers the overlay and opens the matching History row, including pending and failed rows. A right chevron marks this in every visible state. There is no separate error action. Captures without a row open the History list. Opening dismisses that capture's overlay for the rest of its updates.
- History destinations stay queued in Rust until the visible, focused main window selects the History tab and loads the row. Mount and native focus both recover requests.
- Processing dots animate vertically and terminal states fade before the window closes. Hovering pauses dismissal and restores a fading panel, and leaving resumes the remaining time. Hover is sampled from the native cursor and window bounds every 50 ms, without needing focus or WebKit pointer events.
- Reduced Motion skips the transition. A new capture cancels pending fade and hide work, and window opacity resets before reuse.

<!-- os:macos -->
On macOS the whole native window fades over 220 ms, including the Popover material and shadow, and the native window supplies the outer shadow. macOS accepts the first click while the overlay stays non-focused. Other platforms fade the CSS panel. Native dismissal owns both the fade event and the window hide, so they pause together. The fade duration comes from `config/application.jsonc` and is shared with the CSS transition.

## Component inventory

Search here and in the component directory before creating a control.

### Controls

- `Button`: `variant="primary"` for the main action, `outline` for a secondary one, `ghost` for minimal chrome, and `link` for text links that align with surrounding text, such as the About section's legal documents. `link` has no padding or minimum height. All keep normal text at the standard foreground color. The `sm` size is for compact toolbar text actions. `toolbar-icon` shares the 30 px `toolbar-button` height with settings navigation triggers, and `toolbar` gives that height to icon-and-text actions.
- `SplitButton`: a direct action with a chevron menu for alternates. Borderless ghost buttons, with action content, labels, disabled states, and a `menu` snippet made of standard DropdownMenu components. It owns the trigger and popup layout. `divided` adds a separator between the halves, and `actionDisabled` disables only the direct action.
- `DropdownMenuButton`: a chevron-only action menu on the shared dropdown surface. Supply a label and menu snippet. Profile rows and the menu half of `SplitButton` use it.
- `DropdownMenu` (`ui/dropdown-menu`): the adopted shadcn-svelte action-menu components. Compose items, separators, `Group`, and `GroupHeading` for labelled sections. Use Select or Combobox to choose a saved value. Menu snippets use these components directly without repeating styles.
- `Select`: for standardized reference and enum fields, with accessible empty, loading, disabled, and validation states. `SettingsSelect` is the app wrapper.
- `Popover`: for custom interactive popup content. Its primitive owns positioning, focus restoration, Escape, and outside dismissal, and the wrapper uses the shared `menuSurfaceClasses` glass surface with the reduced-transparency fallback.
- `PositionSelect` and `PositionGrid`: a dropdown for nine positions supplied in row order. The trigger shows the selected symbol and label in the shared select trigger style, and a Popover holds the grid, with targets at least 44 px tall. The popup matches the trigger width, and the screen keeps its aspect ratio. Arrow keys move focus, Home and End jump to the first and last position, and Enter, Space, or a click selects, saves, and closes. Opening focuses the selected tile and closing restores the trigger. Only the screen's four outer corners are rounded. The trigger previews the hovered or focused zone. Capture overlay settings use it at a 16 rem field width.
- `Switch` through `SettingsSwitch`: boolean settings. The label activates the accessible switch and disabled styling is kept.
- `Tabs`: the app-wide Basic, Advanced, and Debug mode selector, with all labels visible in the segmented treatment and a moving active indicator.
- `SegmentedControl`: the same sliding selection for named choices. Use the compact default in the footer and `fullWidth` for longer settings labels. An `optionContent` snippet supports custom content. Options can be disabled, and the primitive owns keyboard navigation and skips unavailable choices.
- `ActionInput`: a text or password field with a trailing snippet in one bordered control. It owns the accessible label, focus treatment, and Enter submission, and callers own the trailing icon or action, value, and submit behavior. `variant="flush"` is a compact, square field that spans its container with only a bottom border, for a toolbar row at a panel's edge, and its trailing area fits more than one icon action.
- `ToggleChipGroup`: a wrapping row of independent on/off chips on the bits-ui ToggleGroup primitive (`ui/toggle-group`), for choosing any number of options. It takes and returns option values in option order. Chips that are on use `bg-highlight` with canvas text. `fullWidth` spreads the chips evenly across one row instead of wrapping them. Use `SegmentedControl` when exactly one option applies.
- `DateRangeCalendar`: a one-month range calendar that spans its container, with days filling seven equal columns so a selected range reads as a continuous band. It shows only the weeks the month needs, so its height changes by a row between months, on the bits-ui RangeCalendar primitive (`ui/range-calendar`), taking and returning `YYYY-MM-DD` strings. It reports a range once both ends are chosen, and pressing the first day again makes a one-day range. It never clears itself, so callers offer their own clear action. Days after today are unavailable unless `allowFuture` is set. Selection ends use `bg-highlight` with canvas text and days between them `bg-highlight-surface`. It works the same in every webview, unlike the native date input.
- `ResizableSplitPane`: a vertical two-pane layout on the adopted Resizable (PaneForge) primitives. It takes content snippets, default and minimum heights, and an optional bound divider position. The separator is a thin divider with a rounded bar handle and hover and keyboard focus feedback. It supports dragging and keyboard resizing, and an overlay keeps preview iframes from capturing the drag.
- `ModelPicker`: a model combobox with an action snippet inside the right edge, descriptions, an option indicator snippet, and an optional predicate that filters suggestions while keeping the selected value and display name.
- `ProgressBar`: a compact native progress bar. Callers pass an accessible label and a 0 to 100 value, and the capture overlay and footer show the percentage as text too.

### Layout and shell

- `AppShell`: owns the window tint, prismatic wash, and backdrop blur, fills the window height, and delegates overflow to children. An optional `footer` snippet takes a separate non-shrinking row. It uses a translucent shell when native backdrop support exists and an opaque fallback otherwise.
- `ContentSurface`: owns only the clipped dotted background and fills the available flex space. It accepts `children`, sits under the settings header or around standalone onboarding screens, is non-interactive, and stays behind content without affecting scroll. Never wrap the header or footer in it.
- `SettingsHeader`: the Sora wordmark beside the native macOS controls, optional centered navigation, and optional `trailing` actions. It reserves the native controls' space only on macOS, keeps one row at every width, and is at least 52 px tall with no local fill. On macOS the empty header around the wordmark drags the window. Its `sounds` prop passes the Play sounds setting to the wordmark.
- `Wordmark`: an outlined SVG built from Sora at weight 600, one path per letter, so native alignment does not depend on font metrics. Its viewBox fits the glyphs, overflow stays visible, and it inherits the foreground color. A triple click plays the trailer's staggered letter wave and, when `sound` is set, its three-note chime. Reduced motion keeps the chime and skips the wave.
- `BrandLogo`: the three-layer app mark from `static/logo.svg`, sized by `class`. Each layer spins independently around its vertical axis when clicked, away from the clicked side; repeated clicks add speed, and every spin settles facing forward. Reduced motion disables spinning.
- `OverscrollReveal`: fades its `children` in at the bottom of a `scroller` element while the wheel keeps pushing past the end, then fades out. Place it in the positioned container that holds the scroll area; it stays behind content and ignores pointer events. `SettingsTabs` uses it to show `MakerMark` past the end of the Shortcuts, Providers, and Settings panels.
- `SettingsTabs`: the settings sections and their tab panels. Its tab list fills the header's navigation slot, a `captureActions` snippet fills the trailing slot, and both live under one Tabs root. Every section keeps an icon and the active one also shows its label, animated. Triggers keep a full accessible label and title, and the list scrolls horizontally in narrow windows. The tabs are History, Shortcuts, Profiles, Providers, and Settings.
- `SettingsSection`: groups related settings under a heading. Use `panel` for bordered editor sections and `nested` for dependent settings. The default `plain` layout is the indented Settings presentation.
- `AppFooter`: capture status on the left and the mode control on the right. The indicator is a green ready dot, a green spinner, a yellow model-loading state, or a red capture-error dot, and its detail opens upward on hover and focus.
- `DebugResourceUsage`: Debug-only resident RAM and loaded local model count beside the status. It samples every two seconds while visible and stops on unmount. RAM excludes WebView helpers and is not a GPU total.
- `GeneralSettings`: owns Play sounds, Open at login, capture overlay position, app metadata, initial setup, and the configuration file location. Its About section is flat like the other sections. `AppUpdates` shows the app mark (`BrandLogo`), the version and a fixed status line, with the single update action inline beside them, then the automatic checks switch row and `AppChangelog` in a `Disclosure`. The configuration file and the legal links as `link` buttons follow on the same left edge. The Settings tab icon is a cog.
- `ThirdPartyNotices`: the About-section wrapper that opens bundled license notices and MPL source-access instructions. It composes `Button` and shows an actionable error if the system's default text viewer cannot open the document.

### Status and feedback

- `StatusIndicator`: the canonical compact status icon and label. Success is a green checkmark beside regular foreground text with no pill or border, busy is a spinner, locked is a lock, and error is an alert icon in danger text announced as an alert. `size="xs"` fits a secondary line under a title. Callers own the label.
- `StatusMessage`: `tone="notice"` or `tone="error"` for surfaced state, with `busy` for an operation in progress.
- `Badge`: a small rounded tag beside a heading or label, such as a version's Current or Latest marker. `tone="neutral"` is the default, and `tone="highlight"` uses the active-selection colors to mark the one item that needs attention. Callers own the text, which must stay short.
- `DiagnosticBanner`: a concise message, an optional recovery action, and an error icon in danger colors. Checking states use neutral colors and a polite live region. The caller owns placement and recovery.
- `ActionCard`: an icon heading, status slot, description, optional feedback, action row, and error, spaced with grid gaps. Permission cards compose it with `StatusIndicator` and `Button`, and feature components own the labels and callbacks.
- `CaptureIcon`: the canonical branded capture geometry, with a square central cross and rounded corner strokes. It is decorative, takes a size class, and relies on its control or heading for the accessible label.
- `CaptureProfileButton`: `SplitButton` with region-capture content and the Profile menu, headed "Capture with profile...". It receives the active Profile, readiness, disabled state, and callbacks. Selecting a Profile never starts a capture, and the menu stays available while only the direct action is not ready. The visible name truncates to fit, and the tooltip, accessible label, and menu keep the full name.

### Sidebar tabs

- `SidebarTabs`, `SidebarTabList`, `SidebarTabTrigger`, `SidebarTabContent`: shared vertical tabs for History and Profiles, wrapping the adopted Tabs primitive for keyboard navigation, focus, and panel semantics. Callers supply list labels, row content, selection, and layout. Keep actions outside tab triggers.
- `SidebarTabRow`: the shared selected-row surface, holding a trigger and optional sibling actions. `SidebarTabTrigger` can show a filled or hollow active marker, with its meaning in the accessible label.
- `ExpandableSettingsRow` remains for the Providers pane, which has its own expansion semantics.
- `Disclosure`: a flat show-and-hide control for secondary content inside a plain settings section. The trigger is the label with a chevron at the standard row height, and the content opens below behind the nested left rule. `expanded` is bindable and starts collapsed. Use it instead of a bordered row when the surrounding section is flat, such as About's What's new.

### Finding unused components

Run `bun run check:unused` to list unreachable frontend files with Knip. The SvelteKit plugin finds route entry points and the `$lib` alias, including the capture overlay route, and `knip.ts` uses the Svelte compiler so component imports count as runtime uses. Generated Rust bindings stay out of the report. An index file can keep an unused primitive reachable through a re-export, so check Knip's export report and real references before pruning.

## shadcn-svelte adoption

shadcn-svelte components are copied open code over headless primitives, and they are customized as part of the app.

- Generated files live under `src/lib/components/ui` (`button`, `combobox`, `dropdown-menu`, `popover`, `resizable`, `select`, `separator`, `switch`, `tabs`, and the shared `menu-styles.ts`).
- Feature panes import the app-facing wrappers from `src/lib/components`, which own isoboard labels, composition, sizing, and styling. Panes do not import Bits UI or generated primitives directly.
- One deliberate exception: a feature may pass a snippet of standard DropdownMenu items, groups, and submenus to a shared control such as `SplitButton`. The shared control owns the trigger, popup, and interaction structure.
- Keep Bits UI imports, including type imports, inside `src/lib/components/ui`. App components derive types from local primitives with `ComponentProps`. ESLint enforces the boundary.
- Use the library for behavior-heavy controls where it reduces accessibility, focus, and keyboard maintenance. Keep simple product-specific components app-owned.
- Map the shadcn variables (`background`, `foreground`, `card`, `popover`, `primary`, `secondary`, `accent`, `muted`, `destructive`, `border`, `input`, `ring`, `radius`) to isoboard tokens in `:root`, and do not accept registry default styling. `muted-foreground` maps to `ink` on purpose, so normal text is never dimmed.
- CLI `init`, `add`, and update commands must not overwrite `src/app.css` or other customizations. Review every generated diff first.
- Select, Combobox, and action menus share the glass dropdown surface in `ui/menu-styles.ts`: semi-transparent with a large backdrop blur and readable contrast.
- DropdownMenu was copied from the official [shadcn-svelte registry](https://shadcn-svelte.com/registry/dropdown-menu.json). Its public API, bindings, slots, and composition follow upstream. Surface, item, and status styling use isoboard tokens and the shared menu styles, and the upstream license notice stays beside the components. Review upstream updates instead of overwriting.
- The dialogs and error overlays keep their established surfaces. The translucent treatment of grouped surfaces and resting controls is intentional.

## Profiles and configuration references

Capture shortcuts select existing Profiles through `SettingsSelect`. Profile editing lives in the Profiles pane, and there is no separate Inference Profiles pane.

### Profile tabs

- Profiles and History use the shared `SidebarTabs` family.
- Selecting a tab selects that Profile for capture and shows one editor beside the list. Actions stay outside tab triggers, and each pane scrolls independently. The sidebar has standard-size names with model subtitles, an icon-only Add below the last row, and a chevron menu with Duplicate and Delete in each row. A filled marker shows the active Profile.
- Each editor owns a complete Profile draft and composes the shared model editor for the primary and the independent rules settings. Autosave is transactional and preserves local edits when an external snapshot refreshes unchanged fields.

### Editor structure

`ProcessingProfileEditor` owns the complete draft. `ModelSettingsEditor` edits provider, model, reasoning, and advanced options with no separate name or identity, and it strips snapshot-only download metadata before writing. The editor shows Conversion, then Model, then Rules, in separate panels, with the provider and model in one row and reasoning and advanced options below. The primary editor is always visible, and the rules model nests inside Rules.

- Separate-pass rules offer **Use the same model** and **Use a different model** in a full-width sliding segmented control. The same-model option shows the current model name in monospace on its own line. The different-model option expands a second `ModelSettingsEditor`, and its settings stay in TOML when inactive.
- For an OCR-only primary model, the separate-pass switch is always visible, checked, and disabled, with an explanation that custom rules need a text-capable model. It is presentation only while rules are empty. Adding rules persists the separate pass and shows the independent rules model editor, and clearing them hides it and keeps its saved settings. The same-model option is disabled, and no external model is chosen automatically. Capable primary models keep the normal switch.
- Conversion switches and the Advanced application-mode override stay available. Advanced mode also shows the full conversion prompt template and, with Separate pass on, the full postprocessing template. Each is a collapsed, independently expandable field prefilled with the bundled default. An unchanged source saves as no override, a custom source autosaves when focus leaves the field, and Reset to default clears the override at once (see [Inference and prompts](../inference/#prompt-template-overrides)).
- Profile name and Additional Rules stay local to the draft while typing and commit the whole draft on blur. Conversion switches, mode changes, model settings, and rules model selection save the full validated draft.
- Providers owns shared authentication and endpoints. Local needs no credentials, model suggestions are advisory, and manual IDs stay editable.
- `ConversionOptions`: the shared Tables, Colors, and Images switch group. Callers pass values, disabled states, a layout, and a change callback. Profiles stack the switches vertically and History lays them out horizontally. The profile diagram (`ConversionSkeletonPreview`) scales to fit the switch column.

### Diagnostics

`DiagnosticBanner` shows in a sticky region at the top of the editor pane. Profile tabs keep the active marker and, when capture is blocked, show an error icon and a short danger-colored reason in place of the model subtitle. Checking states use neutral text. Every Profile stays selectable for repair.

## History UI

- The history list is an inset sidebar with space between rows and no separator borders. A rounded surface marks the selected entry.
- The clean preview fills the detail column above the metadata with square edges, on the slightly translucent `surface-preview` token. The isolated document canvas stays transparent so that surface shows through, and explicit HTML backgrounds win.
- Details prioritize the preview above a resizable, scrolling region. They start at four standard metadata rows (180 points with padding) and stop shrinking at one row (60 points). The divider position carries across selections while the browser is mounted, and the preview shrinks to the remaining space in a short window.
- Delete appears in each saved entry's split-button menu. Failed entries use Copy error details as the primary row action. A processing row can disable the primary action without disabling Delete.
- The Options row gives the shared switch group the remaining width beside its label. Preview refreshes keep current content and show progress without changing row height.
- Basic mode hides Additional Rules when none were applied.
- `HistoryFilters` spans the top of the list column as a flush `ActionInput`. Its trailing area holds a clear action, shown while there is search text or any filter, that resets both at once, and the filter button to its right. Enter searches without waiting for the typing pause. The filter button opens a Popover with an Outcome `SegmentedControl`, a Profile `SettingsSelect`, a Contains `ToggleChipGroup`, a `DateRangeCalendar` with the chosen range written above it, and a full-width Clear filters, which resets everything except the search text. The menu is drawn inside the window, so its height is capped at the space below the field and it scrolls in a short window. The button takes `bg-highlight-surface` while any of those filters is active. Matches in a row's text use `bg-highlight-surface`. A filtered list with no rows keeps the controls and says so.

See [Clipboard and history](../clipboard-and-history/) for the data behind these views.

## Local model controls

- `ModelPicker` options show bundle sizes, a primary download icon, ghost trash and cancel icons, and a subtle hard-drive icon centered across both lines of downloaded options, with the tooltip "Downloaded, available offline". Other options reserve the same space.
- `LocalModelControls` reuses `Button` for icon actions with tooltips and `ProgressBar` for transient download feedback. Runtime and sleeping labels and manual unload controls are not shown in Profile editors, because capture owns loading feedback.
- Removing a download uses a native confirmation dialog that names the model and the effect on Profiles. Download progress says users can leave the window and, with notifications enabled, will get a completion notification.
- `ModelInfo` uses the shared icon `Button` to open the selected model's Hugging Face page, with the URL taken from the backend's curated repository identity. It works before and after download and does not change readiness. `ModelLicenseNotice` states that downloading accepts the model's license.
- `local-models-controller.svelte.ts` owns one runtime event subscription in the root page, rejects older snapshot revisions, and shares pending actions and errors across editors. `config-session.svelte.ts` stays the sole config owner and reconciles readiness only when the downloaded state changes, not on progress events.

See [Local models](../local-models/) for the runtime.

## Initial setup

- `SetupProgress` shows the shared ordered setup stages for permissions, the local download card, and the settings guide. Callers pass the current step, the local or external path, an optional manual Model stage, and completed steps.
- `LocalModelSetupStep` owns the focused isOCR card, composing `Button`, `ProgressBar`, and `SetupProgress`. It takes the shared model snapshot and callbacks, and download and configuration ownership stay in their controllers. The local option pairs the recommended model details with a large download or continue tile, and download progress takes a reserved area beside it. Cancel, Resume, Retry, and Continue keep the same footprint. The external provider option is its own full-width row with an icon and a connection action. Narrow windows stack local details and the action, and both use the flat settings-panel surface.
- Permissions and the local card fill the window above the footer. The external and final stages keep the settings navigation and guide.
- Both permission cards use `ActionCard`, with purpose text inside the card, primary request actions as icon buttons, and outline System Settings actions. Screen Recording and Notifications refresh on native window activation, a focus change during a request queues one refresh, and no manual check button exists. Notification permission never gates Continue.
- Screen Recording shows Waiting for access after a request, pointing to System Settings and explaining automatic detection on return. A confirmed grant that is not yet effective in the running process shows Restart needed and Restart isoboard, with "Access is enabled. Restart isoboard to finish."
- The permission step pairs its heading with a shield icon and its access row with `CaptureIcon`. Status uses a checkmark, lock, or busy indicator. Decorative icons are hidden from assistive technology, and busy icons honor reduced motion.

## Licensing UI

- `LicenseSetupStep`: the first-run choice, built from `ChoiceCard` and `Button`. Two columns at supported desktop sizes and stacked below 640 px.
- `ChoiceCard`: a prominent icon, display-font heading, description, optional supporting content, and a bottom-aligned action. Use it in an equal-height grid for parallel options. Callers supply the icon and action snippets and own selection.
- `LicenseSettings`: owns the evaluation and license status in Settings. It shows one permission summary, then a primary Buy a license button with a shopping-cart icon that opens Polar checkout in the default browser (an inline error if it fails), and inline key entry. Verified commercial status replaces the trial copy and purchase controls with the licensed summary, a Priority Email Support status line, an Email support button, and local removal. Privacy sits beside the license documents before activation. Verification errors offer Retry without clearing cached entitlement.
- `LicenseActivation`: `ActionInput` and `Button`. The trailing key icon becomes a primary submit arrow when the field has a key, submission shows a spinner and validates through the Rust licensing controller, and errors appear beside the input. Entry is visible text, trimmed and uppercased, and submit is enabled only for the configured `ISOAPP-` prefix plus the UUID-shaped key. Incomplete input gets a format hint on blur or Enter. Polar stays authoritative. Local removal uses the outline variant.
- `LicenseDocuments` and `BundledDocumentLinks`: `BundledDocumentLinks` is the shared resource-opening control for licensing and third-party notices. Pass resource paths, labels, a disabled state, a button variant, and recovery text. Tauri capabilities must allow each resource path.
- `EvaluationExpiryNotice`: the one-time in-window notice, shown without interrupting capture.

License and third-party document links, the support contact, the checkout destination, trial-day copy, and the key prefix come from `config/product.jsonc` through `product-config.ts`. Do not repeat them in components. Shared editor limits and feedback timing come from `application-config.ts` and `config/application.jsonc`. Setup model selection and provider labels come from `config/model-defaults.jsonc`, and fallback provider capabilities are generated from Rust with the IPC bindings. Component geometry and theme tokens stay local to their components and the stylesheet. See [Licensing](../licensing/) and [Configuration](../configuration/).

## Related

- [Architecture](../architecture/)
- [Configuration](../configuration/)
- [Licensing](../licensing/)
- [Testing and CI](../testing-and-ci/)
