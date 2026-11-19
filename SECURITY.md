# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately. Do not open a public issue.

- Preferred: use **Report a vulnerability** on this repository's
  [Security tab](https://github.com/carlesonielfa/isoboard-app/security/advisories/new).
- Alternatively, email support@isoboard.app with "Security" in the subject.

Include the affected version (Settings → General → About), your macOS version,
the impact, and steps or a proof of concept to reproduce it. Remove real API
keys, license keys and confidential screenshots from anything you send.

You will receive an acknowledgement when the report has been reviewed. Fixes are
released as soon as practical; we will coordinate disclosure timing with you and
credit you in the release notes unless you prefer otherwise. There is no paid
bug bounty.

## Supported versions

Security fixes are made for the latest release only. Please reproduce issues on
the latest version before reporting.

## Scope

In scope is the isoboard desktop app and the code in this repository, including:

- storage of provider API keys and license keys in the macOS Keychain
- network requests the app makes
- local files the app writes: configuration, history and downloaded models
- HTML sanitization of model output before it reaches the clipboard
- model download integrity checks

Out of scope are vulnerabilities in third-party AI providers, Polar, GitHub or
Hugging Face services, and issues that require an already compromised Mac or
user account. Report those to the relevant vendor.
