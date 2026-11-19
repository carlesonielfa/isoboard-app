# Contributing to isoboard

You can inspect, modify and test isoboard, including as paid development work,
under the [source license](LICENSE.md). You do not need to submit your changes.
Productive commercial app use remains subject to that license.

For changes you want included in official releases, we use the
[Contributor Agreement](CONTRIBUTOR-AGREEMENT.md). You keep ownership.
It lets isoboard include your work in its agreed free and paid licensing model,
without giving unrestricted future relicensing rights.

## Preparing a pull request

- Describe the problem, the change and how you checked it. See the
  [README](README.md) for development commands and [AGENTS.md](AGENTS.md) for
  repository conventions.
- Identify everyone whose work is included. Identify third-party material, its
  source and license; preserve its notices. Flag any known rights restrictions.
- If an employer or organization owns relevant rights, arrange authorization
  before inclusion. Contact [legal@isoboard.app](mailto:legal@isoboard.app) privately
  for identity and authority details. Do not post those documents or personal
  addresses in the pull request.

## Accepting the agreement

The first time you open a pull request, a bot asks each author to accept the
[Contributor Agreement](CONTRIBUTOR-AGREEMENT.md). Read it, then post this
comment on the pull request, exactly as written:

```text
I have read the isoboard Contributor Agreement isoboard-contributor-v1 and I accept it.
```

You only do this once. The bot records the acceptance and covers your later pull
requests. Comment `recheck` if the check does not update.

The link in the bot's comment opens the agreement at the `isoboard-contributor-v1`
tag, which fixes the agreed text and its `isoboard-terms-v2` license package.
If someone else's work is in your pull request, they need to accept too. If an
employer or organization owns rights in your work, email legal@isoboard.app
before the pull request is merged.

Bug reports and discussions do not automatically become licensed contributions
under this agreement. If we want to incorporate text, assets or a patch from them,
we will identify the work and arrange the necessary permission.

The bot stores your GitHub username and ID, the pull request number, and the
comment ID and time in a private repository. Your acceptance comment itself is
public on GitHub. See the [privacy notice](src-tauri/legal/PRIVACY-NOTICE.md) for
how these records are handled; use the licensing email for private details.
