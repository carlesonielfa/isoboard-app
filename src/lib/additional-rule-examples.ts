import {
  BookA,
  CalendarDays,
  Code,
  Coins,
  FileText,
  KeyRound,
  Languages,
  List,
  ListChecks,
  Mail,
  Newspaper,
  Quote,
  Ruler,
  SpellCheck,
  Table2,
  TextWrap,
} from '@lucide/svelte';
import type {
  ExampleGalleryCategory,
  ExampleGalleryItem,
} from '$lib/components/ExampleGallery.svelte';

export type AdditionalRuleExample = ExampleGalleryItem & {
  /** Full instruction appended to Additional Rules. */
  text: string;
};

export const additionalRuleExampleCategories: readonly ExampleGalleryCategory<AdditionalRuleExample>[] =
  [
    {
      value: 'language',
      label: 'Language',
      items: [
        {
          id: 'translate-english',
          text: 'Translate all text to English.',
          icon: Languages,
          tone: 'violet',
        },
        {
          id: 'fix-spelling',
          text: 'Fix spelling and grammar mistakes without changing the meaning.',
          icon: SpellCheck,
          tone: 'blue',
        },
        {
          id: 'metric-units',
          text: 'Convert imperial units to metric and round to one decimal.',
          icon: Ruler,
          tone: 'rose',
        },
        {
          id: 'expand-abbreviations',
          text: 'Expand abbreviations on first use.',
          icon: BookA,
          tone: 'violet',
        },
      ],
    },
    {
      value: 'clean-up',
      label: 'Clean up',
      items: [
        {
          id: 'remove-citations',
          text: 'Remove citation numbers such as [1] or (2, 3).',
          icon: Quote,
          tone: 'rose',
        },
        {
          id: 'remove-answer-keys',
          text: 'Remove answer keys and worked solutions, and keep the questions.',
          icon: KeyRound,
          tone: 'violet',
        },
        {
          id: 'remove-page-furniture',
          text: 'Remove page headers, footers and page numbers.',
          icon: FileText,
          tone: 'blue',
        },
        {
          id: 'join-lines',
          text: 'Join lines and hyphenated words broken across lines into normal paragraphs.',
          icon: TextWrap,
          tone: 'rose',
        },
      ],
    },
    {
      value: 'format',
      label: 'Format',
      items: [
        {
          id: 'format-table',
          text: 'Format lists of related values as a table.',
          icon: Table2,
          tone: 'violet',
        },
        {
          id: 'iso-dates',
          text: 'Write dates as year-month-day, for example 2025-03-04.',
          icon: CalendarDays,
          tone: 'blue',
        },
        {
          id: 'bullet-lists',
          text: 'Turn inline lists of items into bullet lists.',
          icon: List,
          tone: 'rose',
        },
        {
          id: 'preserve-code',
          text: 'Preserve code, commands and URLs exactly as written.',
          icon: Code,
          tone: 'violet',
        },
      ],
    },
    {
      value: 'extract',
      label: 'Extract',
      items: [
        {
          id: 'names-emails',
          text: 'Keep only names and email addresses, one person per line.',
          icon: Mail,
          tone: 'blue',
        },
        {
          id: 'main-text',
          text: 'Keep only the main article text, and drop menus, ads and image captions.',
          icon: Newspaper,
          tone: 'rose',
        },
        {
          id: 'action-items',
          text: 'Keep only action items, as a checklist with the owner first.',
          icon: ListChecks,
          tone: 'violet',
        },
        {
          id: 'dates-amounts',
          text: 'Keep only dates and amounts, one per line as date: amount.',
          icon: Coins,
          tone: 'blue',
        },
      ],
    },
  ];

/** Whether a line of `rules` already matches `rule`, ignoring surrounding space and case. */
export function hasAdditionalRule(rules: string, rule: string): boolean {
  const target = rule.trim().toLowerCase();
  return rules.split('\n').some((line) => line.trim().toLowerCase() === target);
}

/** Appends `rule` as a new last line, without leaving blank lines between rules. */
export function appendAdditionalRule(rules: string, rule: string): string {
  if (hasAdditionalRule(rules, rule)) return rules;
  const existing = rules.trimEnd();
  return existing ? `${existing}\n${rule}` : rule;
}
