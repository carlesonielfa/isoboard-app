import { openUrl } from '@tauri-apps/plugin-opener';
import { productConfig } from '$lib/product-config';
import type { InferenceBackend } from '$lib/generated/bindings';

/** User documentation pages, as paths under the docs site root. */
export type DocsPage =
  | 'getting-started/first-capture'
  | 'getting-started/install'
  | 'getting-started/permissions'
  | 'use/additional-rules'
  | 'use/history'
  | 'use/profiles'
  | 'models/choose-a-model'
  | 'models/local-models'
  | 'models/openai'
  | 'models/anthropic'
  | 'models/gemini'
  | 'models/openai-compatible'
  | 'license/activate'
  | 'license/pricing-and-licensing'
  | 'reference/configuration-file'
  | 'reference/keyboard-shortcuts'
  | 'reference/settings';

/** An accessible label and the documentation URL it opens. */
export type DocsLink = { label: string; url: string };

/** Builds a docs URL; `section` is the heading anchor without `#`. The site uses trailing slashes. */
export function docsUrl(page?: DocsPage, section?: string): string {
  const base = productConfig.docsUrl.replace(/\/+$/, '');
  return `${base}/${page ? `${page}/` : ''}${section ? `#${section}` : ''}`;
}

function link(label: string, page?: DocsPage, section?: string): DocsLink {
  return { label, url: docsUrl(page, section) };
}

/** In-app help targets. Section anchors are the docs heading slugs; tests check that they exist. */
export const docsLinks = {
  home: link('Open the isoboard documentation'),
  displayMode: link(
    'Learn about display modes',
    'reference/settings',
    'what-do-basic-advanced-and-debug-do',
  ),

  screenRecording: link(
    'Learn about Screen Recording permission',
    'getting-started/permissions',
    'grant-screen-recording-permission-on-macos',
  ),
  notifications: link(
    'Learn about notifications',
    'getting-started/permissions',
    'allow-notifications-on-macos',
  ),
  isocr: link('Learn about isOCR', 'models/local-models', 'what-is-isocr'),
  chooseModel: link(
    'Learn about local models and AI providers',
    'models/choose-a-model',
    'local-model-or-ai-provider-how-do-they-compare',
  ),
  licenseChoice: link(
    'Learn about personal and commercial use',
    'license/pricing-and-licensing',
    'can-i-use-isoboard-at-work',
  ),
  firstCapture: link(
    'Learn about making a capture',
    'getting-started/first-capture',
    'how-do-i-make-the-capture',
  ),

  launchAtLogin: link(
    'Learn about launch at login',
    'getting-started/permissions',
    'launch-at-login-on-macos',
  ),
  captureStatus: link('Learn about capture status', 'reference/settings', 'capture-status'),
  keepOriginals: link('Learn about original screenshots', 'use/history', 'what-does-history-save'),
  license: link(
    'Learn about licensing',
    'license/pricing-and-licensing',
    'is-isoboard-free-to-use',
  ),
  licenseKey: link(
    'Learn about activating a license key',
    'license/activate',
    'how-do-i-activate-my-license-key',
  ),
  prioritySupport: link(
    'Learn about priority email support',
    'license/pricing-and-licensing',
    'how-long-does-priority-email-support-last',
  ),
  updates: link('Learn about updates', 'getting-started/install', 'how-do-i-update-isoboard'),
  configurationFile: link(
    'Learn about the configuration file',
    'reference/configuration-file',
    'where-is-the-configuration-file',
  ),

  historyOptions: link(
    'Learn about changing options on a saved result',
    'use/history',
    'how-do-i-change-tables-colors-or-images-on-a-saved-result',
  ),
  historyRules: link(
    'Learn about Additional Rules in History',
    'use/additional-rules',
    'can-i-see-whether-rules-were-applied',
  ),

  profileShortcuts: link(
    'Learn about Profile shortcuts',
    'reference/keyboard-shortcuts',
    'how-do-profile-shortcuts-work',
  ),

  profiles: link('Learn about Profiles', 'use/profiles', 'what-does-a-profile-contain'),
  conversionOptions: link(
    'Learn about Tables, Colors and Images',
    'use/profiles',
    'what-do-tables-colors-and-images-do',
  ),
  conversionApplication: link(
    'Learn about conversion settings application',
    'use/profiles',
    'what-does-conversion-settings-application-do',
  ),
  promptTemplates: link(
    'Learn about prompt templates',
    'reference/settings',
    'what-does-advanced-mode-add',
  ),
  profileModel: link(
    'Learn about choosing a model',
    'use/profiles',
    'how-do-i-choose-the-model-for-a-profile',
  ),
  reasoning: link('Learn about reasoning', 'reference/configuration-file', 'options'),
  additionalRules: link(
    'Learn about Additional Rules',
    'use/additional-rules',
    'what-can-additional-rules-do',
  ),
  rulesSeparatePass: link(
    'Learn about applying rules in a separate pass',
    'use/additional-rules',
    'how-are-rules-applied',
  ),
  rulesModel: link(
    'Learn about the rules model',
    'use/additional-rules',
    'which-model-applies-the-rules',
  ),

  endpoint: link(
    'Learn about endpoint rules',
    'models/openai-compatible',
    'what-endpoint-rules-apply',
  ),
} as const satisfies Record<string, DocsLink>;

function providerLinks(name: string, page: DocsPage, connect: string, apiKey: string) {
  return {
    about: link(`Learn about ${name}`, page, connect),
    apiKey: link('Learn where isoboard stores your API key', page, apiKey),
  };
}

/** Per-provider pages: connection steps and where the API key is stored. */
export const providerDocsLinks = {
  local: {
    about: link('Learn about local models', 'models/local-models', 'what-is-isocr'),
    apiKey: null,
  },
  openai_api: providerLinks(
    'OpenAI',
    'models/openai',
    'how-do-i-connect-openai',
    'where-does-isoboard-store-my-key',
  ),
  anthropic_api: providerLinks(
    'Anthropic',
    'models/anthropic',
    'how-do-i-connect-anthropic',
    'where-does-isoboard-store-my-key',
  ),
  gemini_api: providerLinks(
    'Gemini',
    'models/gemini',
    'how-do-i-connect-gemini',
    'where-does-isoboard-store-my-key',
  ),
  openai_compatible: providerLinks(
    'OpenAI-compatible servers',
    'models/openai-compatible',
    'how-do-i-connect-an-openai-compatible-server',
    'where-is-my-api-key-stored',
  ),
} as const satisfies Record<InferenceBackend, { about: DocsLink; apiKey: DocsLink | null }>;

/** Opens a docs URL in the default browser. Resolves to false when the system cannot open it. */
export async function openDocs(url: string): Promise<boolean> {
  try {
    await openUrl(url);
    return true;
  } catch {
    return false;
  }
}
