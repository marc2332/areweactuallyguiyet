import { dashboard } from '../../lib/data.mjs';

export function GET() {
  return new Response(JSON.stringify(dashboard), {
    headers: { 'Content-Type': 'application/json' },
  });
}
