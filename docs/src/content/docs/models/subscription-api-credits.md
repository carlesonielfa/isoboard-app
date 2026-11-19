---
title: Use ChatGPT or Claude plan credits with isoboard
description: ChatGPT Pro, Business, Claude Max, Ultra and Team plans include monthly API credits. Link them to an API key and use them for isoboard captures.
sidebar:
  label: ChatGPT and Claude plans
  order: 5
---

Some ChatGPT and Claude plans include monthly API credits. ChatGPT Pro and Business, and Claude Max, Ultra and Team, give you between $20 and $500 of API usage each month. Link the credits to an API organization, create an API key there, and connect the key in isoboard. Captures then use your plan's credits before any paid balance.

## Which plans include API credits?

Amounts as published by OpenAI and Anthropic in November 2026. Check the official pages linked below for changes.

| Plan | Monthly API credits | Use them with |
|---|---|---|
| ChatGPT Pro | $100, $200 or $500, matching your Pro tier | [OpenAI](../openai/) |
| ChatGPT Business | $20 per Standard seat, $100 per Premium seat, up to $500 per workspace | [OpenAI](../openai/) |
| Claude Max | $100 | [Anthropic](../anthropic/) |
| Claude Ultra | $200 | [Anthropic](../anthropic/) |
| Claude Team | $20 per Standard seat, $100 per Premium seat, up to $500 | [Anthropic](../anthropic/) |

ChatGPT Free, Go, Plus and Enterprise, and Claude Free, Pro and Enterprise, do not include API credits. Free trials and complimentary subscriptions do not qualify. With those plans, add a payment method or prepaid credit to your API account instead.

## How do I use ChatGPT plan credits in isoboard?

You need a ChatGPT Pro or Business plan that has been active for at least 7 days. Use the same email address for ChatGPT and the OpenAI API Platform.

1. Sign in to [chatgpt.com](https://chatgpt.com) with the account that owns the plan.
2. Open **Settings** → **Billing** (Pro), or the workspace billing settings (Business), and claim the **API credits** benefit.
3. Choose the API organization that should receive the credits. You must own it.
4. In that organization, [create an API key](https://platform.openai.com/api-keys).
5. In isoboard, paste the key into the **OpenAI** row on the **Providers** tab. See [use isoboard with an OpenAI API key](../openai/).

Credits arrive again each time the plan renews. You do not need to claim them every month.

## How do I use Claude plan credits in isoboard?

You need a Claude Max, Ultra or Team plan that has been active for at least 7 days. On Team, a Primary Owner or Owner links the credits.

1. Sign in to [claude.ai](https://claude.ai) in a browser.
2. Open **Settings** → **Billing** (Max and Ultra), or **Organization settings** → **Billing** (Team).
3. In **API credits**, select **Link organization** and choose or create a Claude Console organization. You need the Owner, Admin or Billing role in it.
4. In that organization, [create an API key](https://platform.claude.com/settings/keys).
5. In isoboard, paste the key into the **Anthropic** row on the **Providers** tab. See [use isoboard with an Anthropic API key](../anthropic/).

You can link only one Console organization per plan. Changing it later requires contacting Anthropic support.

## What happens when the credits run out?

Unused credits expire at the end of each billing cycle and do not roll over. Plan credits are spent before any credit you bought.

- **OpenAI:** without purchased credit or Auto Recharge, requests fail until the next grant. isoboard shows the capture as failed with a rate-limit or quota error.
- **Anthropic:** without purchased credit or auto-reload, API requests stop until the next monthly credits arrive. Usage is never charged to your Claude plan.

See [fix AI provider errors](../../troubleshooting/provider-errors/) if captures start failing.

## Frequently asked questions

**Can I sign in to isoboard with my ChatGPT or Claude account instead of using an API key?**

No. isoboard connects to OpenAI and Anthropic with an API key. The plan credits reach isoboard through the API organization you link to your plan.

**Does ChatGPT Plus or Claude Pro work with isoboard?**

Yes, with a separate API key and pay-as-you-go API billing. These plans do not include monthly API credits, so captures are billed to your API account.

**Do captures in isoboard use my ChatGPT or Claude chat limits?**

No. API usage draws from the API credits and API balance, which are separate from the message limits in ChatGPT and Claude.

## Official sources

- OpenAI: [API credits for ChatGPT subscribers](https://developers.openai.com/api/docs/guides/chatgpt-api-credits)
- Anthropic: [Monthly API credits for Max, Ultra and Team plans](https://support.claude.com/en/articles/17154008-monthly-api-credits-for-max-ultra-and-team-plans)

## Related

- [Choose a local model or an AI provider](../choose-a-model/)
- [Use isoboard with an OpenAI API key](../openai/)
- [Use isoboard with an Anthropic API key](../anthropic/)
