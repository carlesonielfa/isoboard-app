---
title: isoboard shows an AI provider error
description: Fix isoboard errors from OpenAI, Anthropic, Gemini and compatible servers. Covers invalid keys, denied access, rate limits, timeouts and Keychain prompts.
sidebar:
  label: Provider errors
  order: 4
---

When a capture that uses an AI provider fails, the menu bar status reads **Provider failed** and History keeps the failed attempt with its error message. Find the message below for its cause and fix. Select **Copy error details** on the failed History entry to share the full error with support.

## Check provider access or Connect provider

The footer shows **Profile needs attention** or **Connect provider**, and capture is disabled. The detail reads one of these:

- "Capture model needs a connection in Providers before capture."
- "Capture model provider access is missing. Check the connection in Providers."
- "Choose a model for capture."

**Cause.** The active [Profile](../../use/profiles/) uses a provider that has no saved key, or has no model.

**Fix.** Open the **Providers** tab, expand the provider and enter the **API key**. Then open the **Profiles** tab and choose a model. The status under the key shows **Ready** when isoboard can read it.

For rules steps, the same messages say "Rules model" instead of "Capture model". See [Additional Rules](../../use/additional-rules/).

## provider credentials are not configured

**Cause.** The Profile points to a provider, but no key is saved for it.

**Fix.** Open the **Providers** tab and enter the key. See [OpenAI](../../models/openai/), [Anthropic](../../models/anthropic/), [Gemini](../../models/gemini/) or [OpenAI-compatible servers](../../models/openai-compatible/).

## the API key is invalid, or provider authentication is required

You can see "the API key is invalid" or "provider authentication is required".

**Cause.** The provider rejected the key, the key was revoked or belongs to another account, or isoboard could not read the saved key from the Keychain.

**Fix.**

1. Create a new key on the provider's site. See [OpenAI](https://platform.openai.com/api-keys), [Anthropic](https://platform.claude.com/settings/keys) or [Gemini](https://aistudio.google.com/apikey).
2. Paste it into **API key** on the **Providers** tab and click outside the field.
3. Capture again.

<!-- os:macos -->
If the status under the key says **Locked** or **Unavailable**, the problem is the macOS Keychain. See [the Keychain prompt](#a-keychain-prompt-appears-or-the-key-shows-locked) below.
For an [OpenAI-compatible server](../../models/openai-compatible/) that needs no key, leave **API key (optional)** empty. isoboard then sends requests without a key.

## provider access was denied

**Cause.** Your account, project or key cannot use that model, or the key lacks permission.

**Fix.** Check the model's availability in your provider account, choose a different model in the Profile, or create a key with the right permissions.

## provider inference is rate limited

**Cause.** You hit a rate limit or ran out of credit.

**Fix.** Wait a minute and capture again. Check billing and usage limits in your provider account. Provider charges are between you and the provider. Some ChatGPT and Claude plans include API credits. See [use your ChatGPT or Claude plan credits](../../models/subscription-api-credits/).

## inference timed out

**Cause.** The provider took longer than the Profile's request timeout. The default is 120 seconds.

**Fix.** Try again, or choose a faster model. Raise **Request timeout (seconds)** in the Profile (Advanced mode). Large local servers can need more time. See [OpenAI-compatible servers](../../models/openai-compatible/).

## the inference request failed

**Cause.** isoboard could not reach the provider. The connection is down, a proxy or firewall blocks it, or the endpoint is not running.

**Fix.** Check your connection and that the provider's website loads. For an OpenAI-compatible server, check that the server is running and that the **Endpoint** is correct.

## the provider did not complete the response

**Cause.** The provider returned an error. Typical reasons are a model ID that does not exist, a model that does not accept images, a request that is too large, or a temporary service problem.

**Fix.** Check the model ID in the Profile. Choose a model that accepts images. Try again later if the provider reports an outage.

## the model identifier is invalid

**Cause.** The Profile has an empty or invalid model ID.

**Fix.** Open the **Profiles** tab and choose a model from the picker, or type the exact ID.

## the endpoint is invalid or the endpoint is not allowed

**Cause.** An OpenAI-compatible **Endpoint** must be a valid URL. `http://` works only for `localhost`, `127.0.0.1` and `[::1]`. Any other host needs `https://`. Only the OpenAI-compatible provider accepts a custom endpoint.

**Fix.** Correct the **Endpoint** on the **Providers** tab. See [OpenAI-compatible servers](../../models/openai-compatible/).

## the provider response was malformed or truncated

You can also see "the provider completed without text content", "the provider output is too large" or "the provider returned an unsupported tool call".

**Cause.** The model returned no text, returned too much, or returned something isoboard cannot use. This happens with models that do not suit the task.

**Fix.** Capture again. If it repeats, choose another model in the Profile. A very large result can exceed the output limit, so try a smaller region.

## the inference option ... is not supported by this provider or model

You see this message with an option name, for example "reasoning", "temperature" or "reasoning effort".

**Cause.** The Profile sets an option that the selected provider or model cannot honor. isoboard fails instead of ignoring it.

**Fix.** Open the Profile and set **Reasoning** to **Not set**, or clear **Temperature** (Advanced mode). A reasoning value of On or Off is not supported at this time. See [Profiles](../../use/profiles/).

## the profile needs a model for each active stage

**Cause.** The Profile applies [Additional Rules](../../use/additional-rules/) in a separate pass and one of the two steps has no model.

**Fix.** Open the **Profiles** tab, scroll to **Rules** and choose a model for the rules step.

## A Keychain prompt appears or the key shows Locked

<!-- os:macos -->
isoboard stores API keys in the macOS Keychain. macOS asks for your permission the first time isoboard reads a saved key.

1. When the prompt appears, enter your Mac login password in the macOS dialog and choose **Always Allow**, or **Allow** to be asked again.
2. If you chose **Deny**, capture again with the provider. isoboard asks again.
3. If the status under the key says **Locked**, unlock your login Keychain in **Keychain Access** and try again.
4. If it says **Unavailable**, enter the key again to replace it.

Enter your password only in the macOS dialog. isoboard never asks for it. See [what leaves your Mac](../../privacy/data/).

## Related

- [Choose a local model or an AI provider](../../models/choose-a-model/)
- [OpenAI](../../models/openai/), [Anthropic](../../models/anthropic/) and [Gemini](../../models/gemini/)
- [OpenAI-compatible servers](../../models/openai-compatible/)
- [Profiles](../../use/profiles/)
