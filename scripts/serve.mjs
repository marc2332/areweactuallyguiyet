import { createServer } from 'node:http';
import { readFile, realpath, stat } from 'node:fs/promises';
import { extname, isAbsolute, join, relative, resolve, sep } from 'node:path';

const port = Number(process.argv[2] ?? 8000);
if (process.argv.length > 3 || !Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error('Usage: node scripts/serve.mjs [port between 1 and 65535]');
}
const root = await realpath('dist');
const types = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.gz': 'application/gzip',
};
function isOutsideRoot(path) {
  const location = relative(root, path);
  return location === '..' || location.startsWith(`..${sep}`) || isAbsolute(location);
}
const server = createServer(async (request, response) => {
  if (!['GET', 'HEAD'].includes(request.method)) {
    response.writeHead(405, { Allow: 'GET, HEAD' }).end();
    return;
  }
  let pathname;
  try {
    pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (pathname.includes('\0')) throw new Error('Invalid path');
  } catch {
    response.writeHead(400).end('Bad request');
    return;
  }
  try {
    let path = resolve(root, `.${pathname}`);
    if (isOutsideRoot(path)) {
      response.writeHead(403).end('Forbidden');
      return;
    }
    if ((await stat(path)).isDirectory()) path = join(path, 'index.html');
    path = await realpath(path);
    if (isOutsideRoot(path)) {
      response.writeHead(403).end('Forbidden');
      return;
    }
    if (!(await stat(path)).isFile()) {
      response.writeHead(404).end('Not found');
      return;
    }
    const content = await readFile(path);
    response.writeHead(200, {
      'Content-Type': types[extname(path)] || 'application/octet-stream',
      'Content-Length': content.length,
      'Cache-Control': 'no-store',
      'X-Content-Type-Options': 'nosniff',
    });
    response.end(request.method === 'HEAD' ? undefined : content);
  } catch (error) {
    const status = ['ENOENT', 'ENOTDIR'].includes(error.code) ? 404 : 500;
    response.writeHead(status).end(status === 404 ? 'Not found' : 'Could not read file');
  }
});
server.on('error', error => {
  console.error(`Could not serve the site: ${error.message}`);
  process.exitCode = 1;
});
server.listen(port, '127.0.0.1', () => {
  console.log(`Website available at http://127.0.0.1:${port}`);
});
