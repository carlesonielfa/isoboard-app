---
title: Convert an image file to rich text with isoboard
description: Open a PNG or JPEG file in isoboard and convert it with the same Profiles as a screen capture. Learn the supported formats and size limits.
draft: true
sidebar:
  label: Convert image files
  order: 2
---

isoboard converts image files the same way it converts screen captures. Choose a PNG or JPEG file, and isoboard runs it through the active Profile and puts the result on your clipboard. Use it for images that are already on your Mac, such as a photographed page or a saved screenshot.

## How do I convert an image file?

<!-- TODO(fact): Where does the user start Open image? The app has a native file picker command, but the menu bar menu, the shortcuts list and the window header contain no Open image action in the current build. SPEC and README say it lives in the menu bar and shortcuts. Document the real entry point once confirmed. -->
1. Start **Open image**.
2. In the file picker, select one image file.
3. isoboard shows **Image selected**, then **Processing capture**, then **Copied to clipboard**.
4. Paste into another app with **⌘ Cmd** + **V**. See [paste results into other apps](../paste/).

Dismissing the file picker cancels the action. Nothing changes on your clipboard and no error appears.

## Which files can isoboard convert?

| Limit | Value |
|---|---|
| Formats | PNG and JPEG |
| Files at a time | One |
| File size | Up to about 20 MB |
| Image size | Very large images are rejected |

An image over these limits fails with an error that names the limit. Resize or crop the image and try again.
<!-- TODO(fact): Are other formats (HEIC, WebP, PDF) planned? Not supported today per the file picker filter. -->

## Which Profile does an image file use?

The active Profile, the same one a screen capture uses. Check or change it at the top of the isoboard window, where the **Capture** button shows its name. Tables, Colors, Images and Additional Rules apply as they do to a screen capture. See [set up Profiles](../profiles/).

## Are converted files saved in History?

Yes. Results from files appear in [History](../history/) like any other result. isoboard keeps a small thumbnail and the result, not the original file.

## Frequently asked questions

**Does an image file leave my Mac?**

Only if the active Profile uses an AI provider. With a local model such as isOCR, the file is read and converted on your Mac. See [what leaves your Mac](../../privacy/data/).

**Does converting a file need Screen Recording permission?**

<!-- TODO(fact): Confirm that Open image works without Screen Recording permission, since the footer reports Setup incomplete when permission is missing. -->
Screen Recording permission is for capturing your screen. A file you pick yourself is not a screen capture. Check the status in the footer if the action is unavailable.

## Related

- [Capture part of your screen](../capture/)
- [Set up Profiles](../profiles/)
- [History](../history/)
