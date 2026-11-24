---
title: Use Ollama, LM Studio or other local servers
description: Point isoboard at Ollama, LM Studio or any OpenAI-compatible endpoint to convert screenshots with a vision model you run yourself. Setup and privacy.
sidebar:
  label: Ollama, LM Studio and others
  order: 7
---

isoboard can send captures to any server that speaks the OpenAI chat API, including Ollama and LM Studio. Enter the server's endpoint URL on the **Providers** tab, then type the ID of a vision-capable model in a Profile. A server on your own Mac keeps images on your device.

## How do I connect an OpenAI-compatible server?

1. Start your server and load a model that accepts images.
2. In isoboard, open Settings and select the **Providers** tab.
3. Expand **OpenAI-compatible**.
4. Enter the server's address in **Endpoint**, ending in `/v1`. Ollama typically listens at `http://localhost:11434/v1` and LM Studio at `http://localhost:1234/v1`.
5. If your server requires a key, enter it in **API key (optional)**. Leave the field empty for servers that need none, such as a default Ollama or LM Studio install.
6. Open the **Profiles** tab, set **Provider** to **OpenAI-compatible**, and type the model ID exactly as the server lists it.


isoboard has no bundled model list for this provider, so you always enter the model ID yourself.

## What endpoint rules apply?

- `http://` works only for `localhost`, `127.0.0.1` and `[::1]`. Any other host must use `https://`.

A server on another computer on your network needs HTTPS to work.

## Which models work?

The model must accept images, because isoboard sends the capture as an image. A text-only model fails at capture time. isoboard does not block it when you save. A text-only model can still serve the separate [Additional Rules](../../use/additional-rules/) step if you pick it for that step in a Profile.

## Is it private?

It depends on where the server runs. A server on your own Mac keeps the image on your device, similar to a [local model](../local-models/). A remote server receives the image, and its operator applies its own policies. See [what leaves your Mac](../../privacy/data/).

## Where is my API key stored?

<!-- os:macos -->
In the macOS Keychain, never in the configuration file. A saved key appears as dots in **API key (optional)**. Paste a new key to replace it, or click the trash button next to the field and confirm **Remove key** to delete it from this Mac. The endpoint stays saved. See [where isoboard stores your data](../../privacy/data/).

## Troubleshooting

- Nothing happens or the request fails: check the server is running and the endpoint ends in `/v1`.
- The server rejects the image: the model probably has no vision support.
- Long waits: a large model may take longer than the Profile's timeout, which defaults to 120 seconds. Raise it in the Profile's advanced options.

See also [provider errors](../../troubleshooting/provider-errors/).

## Related

- [Choose a local model or an AI provider](../choose-a-model/)
- [Run isOCR and other local models](../local-models/)
- [Profiles](../../use/profiles/)
