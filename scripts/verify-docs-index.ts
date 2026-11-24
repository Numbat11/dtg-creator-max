#!/usr/bin/env tsx
/**
 * PRD-114 Phase 2 — staleness check for docs/INDEX.md.
 *
 * Regenerates the index in-memory (via generateIndex) and diffs against
 * the committed docs/INDEX.md. Exits non-zero — with a hint to run
 * `npm run generate:docs-index` — if they differ.
 *
 * Wired into pre-commit + CI so a docs/ change that forgets to refresh
 * the index cannot be committed.
 *
 *   cd services/product-service && npx tsx ../../scripts/verify-docs-index.ts
 *   # or: npm run verify:docs-index
 *
 * The only volatile line in the output is `**Last Generated:** <date>`
 * (wall-clock). We normalise it on both sides before comparing so a day
 * rollover on an otherwise-unchanged index doesn't trip a false positive.
 */

import * as fs from 'node:fs';
import { generateIndex, OUTPUT_PATH } from './generate-docs-index';

/** Blank the wall-clock "Last Generated" date so it never causes drift. */
function normalize(md: string): string {
  return md.replace(/^\*\*Last Generated:\*\* .*$/m, '**Last Generated:** <normalized>');
}

const expected = generateIndex();

let actual: string;
try {
  actual = fs.readFileSync(OUTPUT_PATH, 'utf-8');
} catch {
  console.error('✗ docs/INDEX.md does not exist.');
  console.error('  Run `npm run generate:docs-index` and commit the result.');
  process.exit(1);
}

if (normalize(actual) !== normalize(expected.markdown)) {
  console.error('✗ docs/INDEX.md is STALE — it does not match what the generator produces.');
  console.error('  Run `npm run generate:docs-index` and `git add docs/INDEX.md`.');
  process.exit(1);
}

console.log(`✓ docs/INDEX.md is up to date (${expected.totalDocs} docs, ${expected.sections} sections).`);
