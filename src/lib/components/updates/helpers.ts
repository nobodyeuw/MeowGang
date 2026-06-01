import type { ChangelogVersion } from './types';

export function normalizeLabel(value: string | undefined | null) {
  return String(value || '').trim().toLowerCase().replace(/\s+/g, '-');
}

export function formatIssueSeverity(value: string | undefined | null) {
  const normalized = normalizeLabel(value);
  if (normalized === 'low-priority') return 'Low';
  if (normalized === 'no-priority') return 'Low';
  return value || '';
}

export function formatFeaturePriority(value: string | undefined | null) {
  const normalized = normalizeLabel(value);
  if (normalized === 'long-term') return 'Long Term';
  if (normalized === 'no-priority') return 'No Prio';
  if (normalized === 'low-priority') return 'Low Prio';
  if (normalized === 'mid-priority' || normalized === 'medium-priority') return 'Mid Prio';
  if (normalized === 'high-priority') return 'High Prio';
  return value || '';
}

export function formatVersionTag(version: string) {
  return version.startsWith('v') ? version : `v${version}`;
}

export function formatChangelogDate(date: string) {
  const match = date.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (!match) return date;
  return `${match[3]}.${match[2]}.${match[1]}`;
}

export function escapeHtml(value: string) {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

export function renderInlineMarkdown(value: string) {
  return escapeHtml(value).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
}

export function renderReleaseNotes(notes: string) {
  const lines = notes.replace(/\r\n/g, '\n').split('\n');
  const html: string[] = [];
  let listOpen = false;

  const closeList = () => {
    if (listOpen) {
      html.push('</ul>');
      listOpen = false;
    }
  };

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      closeList();
      continue;
    }

    if (trimmed.startsWith('### ')) {
      closeList();
      html.push(`<h3>${renderInlineMarkdown(trimmed.slice(4))}</h3>`);
    } else if (trimmed.startsWith('#### ')) {
      closeList();
      html.push(`<h4>${renderInlineMarkdown(trimmed.slice(5))}</h4>`);
    } else if (trimmed.startsWith('## ')) {
      closeList();
      html.push(`<h3>${renderInlineMarkdown(trimmed.slice(3))}</h3>`);
    } else if (trimmed.startsWith('- ')) {
      if (!listOpen) {
        html.push('<ul>');
        listOpen = true;
      }
      html.push(`<li>${renderInlineMarkdown(trimmed.slice(2))}</li>`);
    } else {
      closeList();
      html.push(`<p>${renderInlineMarkdown(trimmed)}</p>`);
    }
  }

  closeList();
  return html.join('');
}

export function buildVersionDetailsHtml(version: ChangelogVersion) {
  const parts: string[] = [];
  for (const change of version.changes) {
    const type = change.type ?? '';
    const desc = change.description ?? '';
    const det = change.details ?? '';
    parts.push(`<div><strong>${escapeHtml(type)}:</strong> ${escapeHtml(desc)}${det ? `<div style="margin-top:6px;color:var(--md-sys-color-on-surface-variant)">${escapeHtml(det).replace(/\n/g, '<br/>')}</div>` : ''}</div>`);
  }
  return parts.join('<hr style="border:none;border-top:1px solid var(--md-sys-color-outline);margin:8px 0;opacity:0.35;"/>');
}
