import { parseArgs } from 'node:util';
import { importRun } from './history.mjs';
const { values, positionals } = parseArgs({ allowPositionals: true, options: { history: { type: 'string', default: 'site/data/history.json' } } });
if (positionals.length !== 1) throw new Error('Usage: node scripts/collect.mjs RUNNER_HISTORY.json [--history site/data/history.json]');
await importRun(positionals[0], values.history);
console.log(`Imported validated run from ${positionals[0]} into ${values.history}`);
