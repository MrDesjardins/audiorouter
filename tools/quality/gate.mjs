import { appendFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { areas, expectedJobs } from './policy.mjs';

export function evaluateGate(needs) {
  const errors = [];
  if (needs.changes?.result !== 'success') return ['Change classification did not succeed'];
  const flags = {};
  for (const key of [...areas, 'windows', 'rust']) {
    const value = needs.changes.outputs?.[key];
    if (value !== 'true' && value !== 'false') errors.push(`Missing or invalid classification: ${key}`);
    flags[key] = value === 'true';
  }
  // Do not trust inconsistent derived outputs to justify a skipped required job.
  const windows = ['backend', 'contracts', 'ui', 'probe', 'sdk', 'release'].some(key => flags[key]);
  const rust = ['backend', 'contracts', 'probe', 'release'].some(key => flags[key]);
  if (flags.windows !== windows || flags.rust !== rust) errors.push('Inconsistent derived classifications');
  for (const [job, required] of Object.entries(expectedJobs(flags))) {
    const result = needs[job]?.result;
    if (required ? result !== 'success' : !['success', 'skipped'].includes(result)) {
      errors.push(`${job}: expected ${required ? 'success' : 'success or explicitly unaffected skip'}, got ${result ?? 'missing'}`);
    }
  }
  return errors;
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  let errors;
  try { errors = evaluateGate(JSON.parse(process.env.NEEDS_RESULTS ?? 'null')); }
  catch { errors = ['Missing or invalid job results']; }
  const message = errors.length ? `Quality gate failed:\n${errors.map(error => `- ${error}`).join('\n')}` : 'Quality gate passed: all required checks succeeded for this workflow commit.';
  console.log(message);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${message}\n`);
  if (errors.length) process.exitCode = 1;
}