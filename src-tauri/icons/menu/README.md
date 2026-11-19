# Menu artwork

These SVGs are exported from the installed `@lucide/svelte` components and
`src/lib/components/CaptureIcon.svelte` by `bun run menu-icons`. Do not edit the
generated SVGs. Lucide's license is included in `LUCIDE-LICENSE`.

Frontend dev and build commands regenerate them. Commit regenerated assets when
changing the capture component, Lucide version, or menu icon selection so Cargo
builds can use them without Node or Bun.

AppKit loads the SVGs directly at 16 points and treats them as template images.
Lucide strokes use 1.5 units on the 24-unit viewBox, matching the capture artwork.
`src-tauri/src/tray_menu_icons.rs` owns the macOS integration and applies the
artwork whenever the tray menu is installed or refreshed.
