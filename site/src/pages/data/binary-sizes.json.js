import { binarySizes } from '../../lib/data.mjs';

export function GET() {
  return new Response(JSON.stringify(binarySizes), {
    headers: { 'Content-Type': 'application/json' },
  });
}
