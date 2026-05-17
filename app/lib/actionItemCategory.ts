// Sub-Project F — TypeScript mirror of agents::action_items::normalize_category.
// Keep these two implementations aligned: changes here MUST come with
// a matching change in src-tauri/src/agents/action_items.rs.

import type { ActionItemCategory } from '../store/voiceStore';

export type { ActionItemCategory };

const CATEGORY_BUCKETS: { bucket: ActionItemCategory; words: string[] }[] = [
  { bucket: 'follow-up', words: ['follow', 'followup', 'reply', 'response', 'respond'] },
  { bucket: 'decision', words: ['decision', 'decide', 'decided', 'choose', 'choice'] },
  { bucket: 'research', words: ['research', 'learn', 'study', 'investigate', 'explore'] },
  { bucket: 'errand', words: ['errand', 'shopping', 'buy', 'pick', 'groceries'] },
  { bucket: 'work', words: ['work', 'job', 'office', 'client', 'meeting'] },
  { bucket: 'personal', words: ['personal', 'self', 'home', 'family'] },
];

export function normalizeCategory(raw: string | null | undefined): ActionItemCategory {
  if (!raw) return 'other';
  const words = raw
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
  for (const { bucket, words: needles } of CATEGORY_BUCKETS) {
    if (words.some((w) => needles.includes(w))) return bucket;
  }
  return 'other';
}

export const CATEGORY_ORDER: ActionItemCategory[] = [
  'work',
  'decision',
  'follow-up',
  'errand',
  'research',
  'personal',
  'other',
];

export const CATEGORY_LABEL: Record<ActionItemCategory, string> = {
  work: 'Work',
  decision: 'Decisions',
  'follow-up': 'Follow-ups',
  errand: 'Errands',
  research: 'Research',
  personal: 'Personal',
  other: 'Other',
};

export const CATEGORY_ACCENT: Record<ActionItemCategory, string> = {
  work: 'border-blue-500/40 text-blue-300',
  decision: 'border-purple-500/40 text-purple-300',
  'follow-up': 'border-amber-500/40 text-amber-300',
  errand: 'border-emerald-500/40 text-emerald-300',
  research: 'border-cyan-500/40 text-cyan-300',
  personal: 'border-pink-500/40 text-pink-300',
  other: 'border-gray-500/40 text-gray-300',
};

export const PRIORITY_ORDER: Array<'high' | 'medium' | 'low'> = ['high', 'medium', 'low'];
export const PRIORITY_RANK: Record<'high' | 'medium' | 'low', number> = {
  high: 0,
  medium: 1,
  low: 2,
};
