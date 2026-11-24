#!/usr/bin/env tsx
/**
 * PRD-114 — codegen for docs/INDEX.md.
 *
 * Walks docs/** /*.md, reads git log for each file's last-commit date,
 * extracts a one-line description from the file header, and renders the
 * index as Markdown. Run:
 *
 *   npm run generate:docs-index
 *
 * The output is committed. A sibling `verify-docs-index.ts` checks for
 * drift in pre-commit so the index can't go stale.
 *
 * Design choices vs the original PRD-114 plan:
 * - Config lives in `docs/index.config.yaml` (sections, overrides,
 *   cleanup_patterns) — the only hand-edited input. This matches the
 *   PRD-114 spec and keeps the curated copy editable without touching
 *   TypeScript. js-yaml is a scripts/ devDependency.
 * - Byte-identical match with the hand-curated baseline is NOT the goal.
 *   The hand version had paragraph summaries and per-area groupings that
 *   are hard to re-derive. The generated index is more uniform (one
 *   table per folder) but always honest about what's there.
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import * as yaml from 'js-yaml';

// ============================================================
// Config
// ============================================================

// The house stack sets `"type": "module"`, so `__dirname` does not exist — derive it.
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');
const DOCS_DIR = path.join(REPO_ROOT, 'docs');
export const OUTPUT_PATH = path.join(DOCS_DIR, 'INDEX.md');
const CONFIG_PATH = path.join(DOCS_DIR, 'index.config.yaml');

type Section = {
  folder: string; // '' = docs/ root; 'research' = docs/research; 'PRDs/active' = docs/PRDs/active
  title: string;
  description?: string;
  columns?: 'default' | 'replaced-by'; // archived/ uses "Replaced by" instead of "Purpose"
  mode?: 'table' | 'summarized'; // summarized = sub-folder counts + recent edits, for very large folders
};

type CleanupPattern = { pattern: RegExp; label: string; action: string };

/** Shape of docs/index.config.yaml as parsed by js-yaml. */
type RawConfig = {
  sections?: Array<{
    folder: string;
    title: string;
    description?: string;
    columns?: 'default' | 'replaced-by';
    mode?: 'table' | 'summarized';
  }>;
  project_title?: string;
  overrides?: Record<string, string>;
  cleanup_patterns?: Array<{ pattern: string; label: string; action: string }>;
};

// ------------------------------------------------------------
// Load the single hand-edited input: docs/index.config.yaml.
// `folder: "(root)"` is normalised to '' (the docs/ root).
// cleanup_patterns' `pattern` strings are compiled to RegExp.
// ------------------------------------------------------------
const rawConfig = yaml.load(fs.readFileSync(CONFIG_PATH, 'utf-8')) as RawConfig;

const SECTIONS: Section[] = (rawConfig.sections ?? []).map(s => ({
  folder: s.folder === '(root)' ? '' : s.folder,
  title: s.title,
  description: s.description,
  columns: s.columns,
  mode: s.mode,
}));

const PROJECT_TITLE: string = rawConfig.project_title ?? 'Project';

const OVERRIDES: Record<string, string> = rawConfig.overrides ?? {};

const CLEANUP_PATTERNS: CleanupPattern[] = (rawConfig.cleanup_patterns ?? []).map(c => ({
  pattern: new RegExp(c.pattern),
  label: c.label,
  action: c.action,
}));

// ============================================================
// Helpers
// ============================================================

type Entry = {
  /** Path relative to repo root, e.g. `docs/research/VOTE-MECHANICS.md`. */
  fullPath: string;
  /** Path relative to docs/, e.g. `research/VOTE-MECHANICS.md`. */
  relPath: string;
  /** Basename (no folder), e.g. `VOTE-MECHANICS.md`. */
  basename: string;
  /** YYYY-MM-DD, or empty string if untracked. */
  lastUpdated: string;
  /** H1 text from the file, falling back to basename if not present. */
  title: string;
  /** One-line description for the index table. */
  description: string;
  /** Which section this entry belongs to (folder key). */
  section: string;
};

function walkDocs(root: string): string[] {
  const out: string[] = [];
  function recurse(dir: string) {
    const items = fs.readdirSync(dir, { withFileTypes: true });
    for (const item of items) {
      const full = path.join(dir, item.name);
      if (item.isDirectory()) {
        recurse(full);
      } else if (item.isFile() && item.name.endsWith('.md')) {
        out.push(full);
      }
    }
  }
  recurse(root);
  return out.sort();
}

function gitDate(absPath: string): string {
  try {
    const rel = path.relative(REPO_ROOT, absPath);
    const out = execSync(`git log -1 --format=%cs -- "${rel}"`, {
      cwd: REPO_ROOT,
      stdio: ['ignore', 'pipe', 'ignore'],
    }).toString().trim();
    return out;
  } catch {
    return '';
  }
}

function extractTitleAndDescription(absPath: string): { title: string; description: string } {
  let content = '';
  try {
    content = fs.readFileSync(absPath, 'utf-8');
  } catch {
    return { title: path.basename(absPath, '.md'), description: '' };
  }

  const lines = content.split(/\r?\n/);

  // Title = first H1 (# Heading) found.
  let title = '';
  let titleLineIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(/^#\s+(.+?)\s*$/);
    if (m) {
      title = m[1].trim();
      titleLineIdx = i;
      break;
    }
  }
  if (!title) title = path.basename(absPath, '.md');

  // Description = first paragraph after the H1 that isn't a metadata line
  // (no leading **, no leading >, no leading -, no leading |).
  let description = '';
  for (let i = titleLineIdx + 1; i < lines.length && i < titleLineIdx + 50; i++) {
    const raw = lines[i].trim();
    if (!raw) continue;
    if (raw.startsWith('**') || raw.startsWith('>') || raw.startsWith('-') || raw.startsWith('|')) continue;
    if (raw.startsWith('#')) continue; // hit another heading without prose
    if (raw.startsWith('```')) continue;
    if (raw.startsWith('---')) continue;
    description = raw;
    break;
  }
  // Strip markdown links to bare text: [foo](bar) → foo
  description = description.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
  // Collapse whitespace
  description = description.replace(/\s+/g, ' ').trim();
  // Truncate
  if (description.length > 180) description = description.slice(0, 177) + '...';

  return { title, description };
}

function sectionFor(relPath: string): string {
  // relPath is relative to docs/, like `research/VOTE-MECHANICS.md` or `BUGS_BUNNY.md`
  if (!relPath.includes('/')) return '';
  if (relPath.startsWith('PRDs/active/')) return 'PRDs/active';
  if (relPath.startsWith('PRDs/completed/')) return 'PRDs/completed';
  if (relPath.startsWith('PRDs/pending/')) return 'PRDs/pending';
  if (relPath.startsWith('PRDs/templates/')) return 'PRDs/templates';
  if (relPath.startsWith('PRDs/')) return 'PRDs'; // PRDs/README.md, 00_decision_log.md → own section
  const top = relPath.split('/')[0];
  return top;
}

function isCleanupCandidate(relPath: string): { label: string; action: string } | null {
  for (const c of CLEANUP_PATTERNS) {
    if (c.pattern.test(relPath)) return { label: c.label, action: c.action };
  }
  return null;
}

function mdLink(text: string, urlRel: string): string {
  // URL-encode just spaces, parens, and commas — keep readable
  const encoded = urlRel
    .replace(/ /g, '%20')
    .replace(/\(/g, '%28')
    .replace(/\)/g, '%29')
    .replace(/,/g, '%2C');
  return `[${text}](${encoded})`;
}

// ============================================================
// Render helper (pure — depends only on its args + mdLink)
// ============================================================

function renderSection(section: Section, sectionEntries: Entry[]): string {
  if (sectionEntries.length === 0) return '';

  const parts: string[] = [];
  parts.push(`## ${section.title} (${sectionEntries.length})`);
  if (section.description) {
    parts.push('');
    parts.push(section.description);
  }
  parts.push('');

  if (section.mode === 'summarized') {
    // Summarized sections get sub-folder counts + recent edits, not a full table.
    const subfolders = new Map<string, Entry[]>();
    for (const e of sectionEntries) {
      const parts2 = e.relPath.split('/');
      const sub = parts2.length > 2 ? parts2[1] : '(root)';
      if (!subfolders.has(sub)) subfolders.set(sub, []);
      subfolders.get(sub)!.push(e);
    }
    parts.push('| Sub-folder | Docs |');
    parts.push('|------------|-----:|');
    for (const [name, files] of Array.from(subfolders.entries()).sort()) {
      parts.push(`| ${name === '(root)' ? '(root)' : `${section.folder}/${name}/`} | ${files.length} |`);
    }
    parts.push('');
    parts.push('**Five most recent edits in this corpus:**');
    parts.push('');
    parts.push('| Date | Document |');
    parts.push('|------|----------|');
    const recent = [...sectionEntries]
      .filter(e => e.lastUpdated)
      .sort((a, b) => (a.lastUpdated < b.lastUpdated ? 1 : -1))
      .slice(0, 5);
    for (const e of recent) {
      parts.push(`| ${e.lastUpdated} | ${mdLink(e.relPath, e.relPath)} |`);
    }
    return parts.join('\n');
  }

  // Default table mode — sort by lastUpdated desc, then by basename.
  const sorted = [...sectionEntries].sort((a, b) => {
    if (a.lastUpdated && b.lastUpdated) {
      if (a.lastUpdated > b.lastUpdated) return -1;
      if (a.lastUpdated < b.lastUpdated) return 1;
    } else if (a.lastUpdated) {
      return -1;
    } else if (b.lastUpdated) {
      return 1;
    }
    return a.basename.localeCompare(b.basename);
  });

  if (section.columns === 'replaced-by') {
    parts.push('| Document | Last Updated | Replaced by |');
    parts.push('|----------|--------------|-------------|');
  } else {
    parts.push('| Document | Last Updated | Purpose |');
    parts.push('|----------|--------------|---------|');
  }
  for (const e of sorted) {
    const date = e.lastUpdated || '_(untracked)_';
    const desc = e.description || '_(no description)_';
    parts.push(`| ${mdLink(e.basename, e.relPath)} | ${date} | ${desc} |`);
  }
  return parts.join('\n');
}

// ============================================================
// Build + assemble. Exported so verify-docs-index.ts can
// regenerate in-memory and diff without writing to disk.
// `generatedDate` is injected (not read from the clock) so the
// verifier can reproduce a byte-identical body — the only
// wall-clock value in the output is the "Last Generated" line.
// ============================================================

export type IndexResult = {
  markdown: string;
  totalDocs: number;
  sections: number;
  cleanupFlagged: number;
  mostRecent?: Entry;
};

export function generateIndex(generatedDate: string = new Date().toISOString().slice(0, 10)): IndexResult {
  const entries: Entry[] = walkDocs(DOCS_DIR)
    .filter(absPath => path.relative(DOCS_DIR, absPath) !== 'INDEX.md') // don't index ourselves
    .map((absPath): Entry => {
      const relPath = path.relative(DOCS_DIR, absPath); // e.g. research/VOTE-MECHANICS.md
      const fullPath = path.relative(REPO_ROOT, absPath); // e.g. docs/research/VOTE-MECHANICS.md
      const basename = path.basename(absPath);
      const lastUpdated = gitDate(absPath);
      const { title, description: extracted } = extractTitleAndDescription(absPath);
      const description = OVERRIDES[fullPath] ?? extracted;
      return {
        fullPath,
        relPath,
        basename,
        lastUpdated,
        title,
        description,
        section: sectionFor(relPath),
      };
    });

  const cleanupEntries = entries
    .map(e => ({ entry: e, cleanup: isCleanupCandidate(e.relPath) }))
    .filter((x): x is { entry: Entry; cleanup: { label: string; action: string } } => x.cleanup !== null);

  const sectionBlocks: string[] = [];
  const seenSections = new Set<string>();
  for (const section of SECTIONS) {
    const sectionEntries = entries.filter(e => e.section === section.folder);
    if (sectionEntries.length === 0) continue;
    seenSections.add(section.folder);
    const block = renderSection(section, sectionEntries);
    if (block) sectionBlocks.push(block);
  }

  // Catch-all for any folder we forgot to declare.
  const unclassified = entries.filter(e => !seenSections.has(e.section));
  if (unclassified.length > 0) {
    sectionBlocks.push(
      renderSection(
        {
          folder: '_other',
          title: 'Other docs (auto-classified)',
          description: 'Files in folders not declared in `docs/index.config.yaml`. Add a section there for nicer grouping.',
        },
        unclassified,
      ),
    );
  }

  // "What's been updated this week" — top 15 most recently committed files.
  const recentWeek = [...entries]
    .filter(e => e.lastUpdated)
    .sort((a, b) => (a.lastUpdated < b.lastUpdated ? 1 : -1))
    .slice(0, 15);

  const recentBlock: string[] = [
    '## What\'s been updated this week (top 15)',
    '',
    'The freshest activity — useful for "what\'s on the founder\'s mind right now?":',
    '',
    '| Date | Document | One-liner |',
    '|------|----------|-----------|',
  ];
  for (const e of recentWeek) {
    recentBlock.push(`| ${e.lastUpdated} | ${mdLink(e.relPath, e.relPath)} | ${e.description || '_(no description)_'} |`);
  }

  // Cleanup section
  const cleanupBlocks: string[] = [];
  if (cleanupEntries.length > 0) {
    cleanupBlocks.push('## 🧹 Cleanup findings');
    cleanupBlocks.push('');
    cleanupBlocks.push('Auto-detected files that look like noise. Worth a small grooming PR.');
    cleanupBlocks.push('');
    const grouped = new Map<string, { entries: Entry[]; action: string }>();
    for (const { entry, cleanup } of cleanupEntries) {
      if (!grouped.has(cleanup.label)) {
        grouped.set(cleanup.label, { entries: [], action: cleanup.action });
      }
      grouped.get(cleanup.label)!.entries.push(entry);
    }
    for (const [label, { entries: files, action }] of grouped.entries()) {
      cleanupBlocks.push(`### ${label} (${files.length})`);
      cleanupBlocks.push('');
      cleanupBlocks.push('```');
      for (const e of files) {
        cleanupBlocks.push(e.fullPath);
      }
      cleanupBlocks.push('```');
      cleanupBlocks.push('');
      cleanupBlocks.push(`Recommended: ${action}.`);
      cleanupBlocks.push('');
    }
  }

  const header = `# ${PROJECT_TITLE} Docs Index

> **AUTO-GENERATED by \`scripts/generate-docs-index.ts\` — DO NOT EDIT BY HAND.**
> Regenerate with: \`npm run generate:docs-index\`
> Drift is blocked at pre-commit by \`scripts/verify-docs-index.ts\`.

**Last Generated:** ${generatedDate}
**Total Documents:** ${entries.length}

---

## How to use this index

- **"Last Updated"** is the latest git commit date for that file — a more honest signal than file \`mtime\` (which gets reset on checkout).
- Tables are grouped by **folder**, then sorted newest-first within each section.
- For onboarding, start with [KICKOFF-BRIEF.md](KICKOFF-BRIEF.md) → [NEXT-SESSION.md](NEXT-SESSION.md) → [PRDs/00_decision_log.md](PRDs/00_decision_log.md) → [BUGS_BUNNY.md](BUGS_BUNNY.md).
- Active work lives in **[PRDs/active/](PRDs/active/)**; ship-and-archive workflow drops finished specs into **[PRDs/completed/](PRDs/completed/)**.
- Cleanup candidates (duplicate \`* 1.md\` macOS conflict files) are listed at the end.

---

`;

  const markdown = [
    header,
    recentBlock.join('\n'),
    '',
    '---',
    '',
    sectionBlocks.join('\n\n---\n\n'),
    cleanupBlocks.length > 0 ? '\n\n---\n\n' + cleanupBlocks.join('\n') : '',
    '\n---\n',
    '## How to keep this index fresh',
    '',
    'This file is generated. Workflow:',
    '',
    '1. Add, rename, move, or delete docs under `docs/`',
    '2. `npm run generate:docs-index` — refreshes this file',
    '3. `git add docs/INDEX.md` — commit alongside your other changes',
    '',
    'Pre-commit blocks commits where `docs/` changed but `docs/INDEX.md` is stale.',
    '',
    '**Source:** [`scripts/generate-docs-index.ts`](../scripts/generate-docs-index.ts) (walker) + [`docs/index.config.yaml`](index.config.yaml) (config)',
    '',
  ].join('\n');

  return {
    markdown,
    totalDocs: entries.length,
    sections: sectionBlocks.length,
    cleanupFlagged: cleanupEntries.length,
    mostRecent: recentWeek[0],
  };
}

// Run directly → write the file + log. Imported (by verify) → no side effects.
// ES-module equivalent of `require.main === module` (`require` doesn't exist under "type": "module").
const runDirectly = process.argv[1] !== undefined && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (runDirectly) {
  const result = generateIndex();
  fs.writeFileSync(OUTPUT_PATH, result.markdown, 'utf-8');
  console.log(`✓ Wrote ${path.relative(REPO_ROOT, OUTPUT_PATH)}`);
  console.log(`  Total docs:       ${result.totalDocs}`);
  console.log(`  Sections:         ${result.sections}`);
  console.log(`  Cleanup flagged:  ${result.cleanupFlagged}`);
  console.log(`  Most recent edit: ${result.mostRecent?.lastUpdated} (${result.mostRecent?.relPath})`);
}
