#!/usr/bin/env node
/**
 * Vis launcher — single-command runner for the OpenCode Visualizer.
 *
 * Modes:
 *   1. Local mode (default): spawns a local opencode server via the
 *      official @opencode-ai/sdk, reverse-proxies /api/* to it, serves
 *      the SPA from the same origin (so no CORS configuration is needed),
 *      and forwards WebSocket upgrades for /api/pty/<id>/connect.
 *   2. Remote mode (--no-spawn / --opencode-url): assumes opencode is
 *      already running somewhere and just proxies to it.
 *   3. Static mode (--static-only): old behaviour — only serves dist/.
 *   4. Proxy mode (proxy <url>): old behaviour — reverse-proxy a hosted UI.
 *
 * CLI:
 *   vis                                     # spawn local opencode + serve UI
 *   vis --port 3000                         # change UI port
 *   vis --opencode-port 4096                # use a fixed opencode port
 *   vis --no-spawn                          # do not start opencode, use existing
 *   vis --opencode-url http://host:4096     # remote opencode server
 *   vis --static-only                       # legacy static-only behaviour
 *   vis proxy https://xenodrive.github.io/vis  # legacy proxy behaviour
 *   vis --help
 */

import { serveStatic } from '@hono/node-server/serve-static';
import { serve } from '@hono/node-server';
import { Hono } from 'hono';
import { proxy } from 'hono/proxy';
import { spawn } from 'node:child_process';
import { createConnection } from 'node:net';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

function parseArgs(argv) {
  const args = {
    mode: 'local',
    port: Number(process.env.VIS_PORT) || 3000,
    opencodePort: Number(process.env.VIS_OPENCODE_PORT) || 0,
    opencodeUrl: process.env.VIS_OPENCODE_URL || '',
    proxyTarget: '',
    help: false,
  };

  // Legacy positional `proxy <url>` mode preserved.
  if (argv[0] === 'proxy') {
    args.mode = 'proxy';
    args.proxyTarget = argv[1] ?? 'https://xenodrive.github.io/vis';
    return args;
  }
  if (argv[0] === 'static' || argv[0] === '--static-only') {
    args.mode = 'static';
    return args;
  }

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const next = () => argv[++i];
    if (arg === '--help' || arg === '-h') args.help = true;
    else if (arg === '--port') args.port = Number(next());
    else if (arg.startsWith('--port=')) args.port = Number(arg.slice(7));
    else if (arg === '--opencode-port') args.opencodePort = Number(next());
    else if (arg.startsWith('--opencode-port=')) args.opencodePort = Number(arg.slice(16));
    else if (arg === '--opencode-url') {
      args.opencodeUrl = next();
      args.mode = 'remote';
    } else if (arg.startsWith('--opencode-url=')) {
      args.opencodeUrl = arg.slice(15);
      args.mode = 'remote';
    } else if (arg === '--no-spawn') args.mode = 'remote';
    else if (arg === '--static-only') args.mode = 'static';
    else if (!arg.startsWith('-')) {
      // Unknown positional — ignore (preserves potential future subcommands).
    }
  }

  return args;
}

function printHelp() {
  process.stdout.write(`
Vis — OpenCode Visualizer launcher

Usage:
  vis                                Start in local mode (spawn opencode, serve UI)
  vis --port 3000                    Change UI port (default: 3000)
  vis --opencode-port 4096           Use a fixed opencode port (default: random free)
  vis --no-spawn                     Don't spawn opencode, expect one already running
  vis --opencode-url http://host:port
                                     Connect to a remote opencode server
  vis --static-only                  Legacy: serve dist/ only (no proxy)
  vis proxy https://xenodrive.github.io/vis
                                     Legacy: reverse-proxy a hosted UI
  vis --help                         Show this help

Environment variables:
  VIS_PORT, VIS_OPENCODE_PORT, VIS_OPENCODE_URL

In local mode the UI is reachable at http://localhost:<port> and OpenCode
is reverse-proxied at <ui>/api/* — no CORS configuration required.
`);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function log(level, ...parts) {
  const stamp = new Date().toISOString().slice(11, 19);
  process.stderr.write(`[vis ${stamp}] ${level} ${parts.join(' ')}\n`);
}

/**
 * Health-check a remote opencode server before we start serving the UI,
 * so the user gets an immediate error instead of a confusing UI login loop.
 */
async function probeOpencode(baseUrl, timeoutMs = 5000) {
  const url = baseUrl.replace(/\/+$/, '') + '/path';
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(url, { signal: AbortSignal.timeout(1000) });
      if (res.ok) return true;
      lastError = new Error(`HTTP ${res.status}`);
    } catch (err) {
      lastError = err;
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error(`opencode unreachable at ${baseUrl} — ${lastError?.message ?? '?'}`);
}

/**
 * Race a promise against a timeout. Returns the resolved value or 'timeout'.
 */
function raceTimeout(promise, ms) {
  return Promise.race([
    promise.then(() => 'done'),
    new Promise((resolve) => setTimeout(() => resolve('timeout'), ms)),
  ]);
}

// ---------------------------------------------------------------------------
// opencode child-process management
// ---------------------------------------------------------------------------
//
// We deliberately spawn opencode ourselves rather than using the SDK's
// `createOpencodeServer` helper. The SDK's stop() implementation calls a
// plain `proc.kill()` on POSIX (see @opencode-ai/sdk/dist/process.js),
// which sends SIGTERM to the root pid only — and opencode itself doesn't
// reliably react to that signal, leaving zombie LSP children behind
// (sst/opencode#20899). By spawning with `detached: true` we make the
// child the leader of its own process group, which lets us SIGTERM the
// whole tree via `process.kill(-pid, 'SIGTERM')` (POSIX) and fall back to
// SIGKILL if it doesn't react in time. The actual launch+healthcheck logic
// mirrors what the SDK does internally so behaviour stays consistent.
//

/**
 * Spawn `opencode serve` as a child process and resolve once it has
 * announced its listening URL on stdout. Returns the child handle plus
 * the parsed URL so the caller can wire up a reverse-proxy.
 */
function spawnOpencode({ port = 0, hostname = '127.0.0.1', timeoutMs = 10000 } = {}) {
  return new Promise((resolve, reject) => {
    const args = ['serve', `--hostname=${hostname}`, `--port=${port}`];
    log('info', `Spawning: opencode ${args.join(' ')}`);

    const child = spawn('opencode', args, {
      // POSIX: lead our own process group so we can group-kill the whole
      // tree at shutdown. The Node parent stays unaffected because we
      // explicitly target -child.pid, never -process.pid.
      detached: process.platform !== 'win32',
      // We need stdout to parse the listening URL; mirror stderr through.
      stdio: ['ignore', 'pipe', 'inherit'],
    });

    let buffer = '';
    let settled = false;

    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      stopOpencodeProcess(child, 'SIGKILL');
      reject(new Error(`Timed out waiting for opencode after ${timeoutMs}ms`));
    }, timeoutMs);

    child.stdout.on('data', (chunk) => {
      const text = chunk.toString();
      // Mirror opencode logs to our stderr so the user can see them.
      process.stderr.write(text);
      if (settled) return;
      buffer += text;
      const match = buffer.match(/opencode server listening on\s+(https?:\/\/[^\s]+)/);
      if (match) {
        settled = true;
        clearTimeout(timer);
        resolve({ child, url: match[1] });
      }
    });

    child.once('error', (err) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(err);
    });

    child.once('exit', (code, signal) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(new Error(`opencode exited before ready (code=${code} signal=${signal})`));
    });
  });
}

/**
 * Send a signal to opencode's entire process group. Falls back to a direct
 * kill on Windows (taskkill /T /F) or if the negative-PID call fails.
 */
function stopOpencodeProcess(child, signal = 'SIGTERM') {
  if (!child || child.killed) return;
  if (child.exitCode !== null || child.signalCode) return;
  const pid = child.pid;
  if (!pid) return;
  if (process.platform === 'win32') {
    try {
      spawn('taskkill', ['/pid', String(pid), '/T', '/F'], { windowsHide: true });
    } catch {
      try { child.kill(signal); } catch { /* ignore */ }
    }
    return;
  }
  try {
    process.kill(-pid, signal);
  } catch (err) {
    if (err && err.code !== 'ESRCH') {
      try { child.kill(signal); } catch { /* ignore */ }
    }
  }
}

/**
 * Wait for a child to actually exit, with a timeout fallback.
 */
function waitForChildExit(child, ms) {
  return new Promise((resolve) => {
    if (child.exitCode !== null || child.signalCode) {
      resolve('exited');
      return;
    }
    const timer = setTimeout(() => resolve('timeout'), ms);
    child.once('exit', () => {
      clearTimeout(timer);
      resolve('exited');
    });
  });
}

// ---------------------------------------------------------------------------
// Hono app builders
// ---------------------------------------------------------------------------

function buildStaticApp() {
  const app = new Hono();
  app.use('*', serveStatic({ root: join(__dirname, 'dist/') }));
  return app;
}

function buildLegacyProxyApp(baseURL) {
  log('info', `Legacy proxy mode → ${baseURL}`);
  const app = new Hono();
  app.use('*', (c) => {
    const url = new URL(baseURL);
    url.pathname = url.pathname.replace(/\/$/, '') + c.req.path;
    const q = c.req.queries();
    for (const k in q) {
      for (const v of q?.[k] ?? []) {
        url.searchParams.append(k, v);
      }
    }
    return proxy(url, { ...c.req });
  });
  return app;
}

async function loadIndexHtml() {
  try {
    return await readFile(join(__dirname, 'dist', 'index.html'), 'utf8');
  } catch (err) {
    log('error', `Failed to read dist/index.html: ${err.message}`);
    log('error', "Did you forget to run 'pnpm build'?");
    return null;
  }
}

function injectApiBase(html, apiBase) {
  // Replace <meta name="vis-api-base" content="..." /> with the launcher's value.
  const re = /<meta\s+name="vis-api-base"\s+content="[^"]*"\s*\/?>/i;
  const replacement = `<meta name="vis-api-base" content="${apiBase}" />`;
  if (re.test(html)) return html.replace(re, replacement);
  // Fallback: inject before </head>.
  return html.replace(/<\/head>/i, `  ${replacement}\n</head>`);
}

function buildLauncherApp(opencodeUrl) {
  log('info', `Same-origin launcher: UI + /api/* → ${opencodeUrl}`);
  const app = new Hono();
  const upstream = opencodeUrl.replace(/\/+$/, '');

  // /api/* — reverse-proxy to opencode (strip the /api prefix).
  app.all('/api/*', async (c) => {
    const rest = c.req.path.replace(/^\/api/, '') || '/';
    const target = new URL(upstream + rest);
    const incoming = new URL(c.req.url);
    target.search = incoming.search;

    // Build a clean header set; drop hop-by-hop and host headers.
    const headers = new Headers(c.req.raw.headers);
    headers.delete('host');
    headers.delete('connection');
    headers.delete('content-length');

    return proxy(target, {
      method: c.req.method,
      headers,
      body: c.req.raw.body,
      // hono/proxy passes through SSE streams correctly.
    });
  });

  // Serve the SPA index with vis-api-base injected to '/api'.
  // We do this for both '/' and '/index.html' so the SPA boots without
  // requiring the user to fill in a baseUrl in the login form.
  let cachedHtml = null;
  const getHtml = async () => {
    if (cachedHtml) return cachedHtml;
    const raw = await loadIndexHtml();
    if (!raw) return null;
    cachedHtml = injectApiBase(raw, '/api');
    return cachedHtml;
  };
  const indexHandler = async (c) => {
    const html = await getHtml();
    if (!html) return c.text('Vis: dist/index.html not found. Run `pnpm build`.', 500);
    return c.html(html);
  };
  app.get('/', indexHandler);
  app.get('/index.html', indexHandler);

  // Everything else → static SPA.
  app.use('*', serveStatic({ root: join(__dirname, 'dist/') }));
  return app;
}

// ---------------------------------------------------------------------------
// WebSocket upgrade proxy (for /api/pty/<id>/connect)
// ---------------------------------------------------------------------------

function attachWsUpgradeProxy(httpServer, opencodeUrl) {
  const upstreamUrl = new URL(opencodeUrl);
  const upstreamHost = upstreamUrl.hostname;
  const upstreamPort = Number(upstreamUrl.port) || 80;

  httpServer.on('upgrade', (clientReq, clientSocket, clientHead) => {
    if (!clientReq.url || !clientReq.url.startsWith('/api/')) {
      clientSocket.destroy();
      return;
    }

    const upstreamPath = clientReq.url.replace(/^\/api/, '') || '/';
    log('info', `WS upgrade ${clientReq.url} → ${upstreamHost}:${upstreamPort}${upstreamPath}`);

    // Sanitize headers (drop Host, set upstream Host).
    const headerLines = [];
    headerLines.push(`GET ${upstreamPath} HTTP/1.1`);
    const rawHeaders = clientReq.rawHeaders;
    let hostSet = false;
    for (let i = 0; i < rawHeaders.length; i += 2) {
      const key = rawHeaders[i];
      const value = rawHeaders[i + 1];
      if (key.toLowerCase() === 'host') {
        headerLines.push(`Host: ${upstreamHost}:${upstreamPort}`);
        hostSet = true;
      } else {
        headerLines.push(`${key}: ${value}`);
      }
    }
    if (!hostSet) headerLines.push(`Host: ${upstreamHost}:${upstreamPort}`);
    headerLines.push('', '');
    const handshake = headerLines.join('\r\n');

    const upstreamSocket = createConnection(
      { host: upstreamHost, port: upstreamPort },
      () => {
        upstreamSocket.write(handshake);
        if (clientHead && clientHead.length > 0) upstreamSocket.write(clientHead);
        upstreamSocket.pipe(clientSocket);
        clientSocket.pipe(upstreamSocket);
      },
    );

    const onError = (label) => (err) => {
      log('warn', `WS proxy ${label} error: ${err?.message ?? err}`);
      clientSocket.destroy();
      upstreamSocket.destroy();
    };
    upstreamSocket.on('error', onError('upstream'));
    clientSocket.on('error', onError('client'));
  });
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    printHelp();
    return;
  }

  let app;
  let opencodeChild = null; // Node ChildProcess we spawned ourselves.
  let opencodeUrl = '';
  let shuttingDown = false;

  if (args.mode === 'proxy') {
    app = buildLegacyProxyApp(args.proxyTarget);
  } else if (args.mode === 'static') {
    log('info', 'Static-only mode — serving dist/');
    app = buildStaticApp();
  } else if (args.mode === 'remote') {
    if (!args.opencodeUrl) {
      args.opencodeUrl = `http://127.0.0.1:${args.opencodePort || 4096}`;
    }
    opencodeUrl = args.opencodeUrl;
    log('info', `Remote mode — proxying to ${opencodeUrl}`);
    try {
      await probeOpencode(opencodeUrl, 5000);
      log('info', `Connected to ${opencodeUrl}`);
    } catch (err) {
      log('warn', `Could not reach ${opencodeUrl}: ${err.message}`);
      log('warn', 'Continuing anyway — UI will retry when opencode is up.');
    }
    app = buildLauncherApp(opencodeUrl);
  } else {
    // Local mode — spawn opencode ourselves with proper process-group
    // semantics so we can group-kill the whole tree at shutdown.
    try {
      const result = await spawnOpencode({
        hostname: '127.0.0.1',
        port: args.opencodePort || 0,
      });
      opencodeChild = result.child;
      opencodeUrl = result.url;
    } catch (err) {
      log('error', `Failed to start opencode: ${err.message}`);
      log('error', "Make sure 'opencode' is installed and on PATH.");
      process.exit(1);
    }
    log('info', `OpenCode is up at ${opencodeUrl}`);
    // If opencode dies on its own, take the launcher down too.
    opencodeChild.once('exit', (code, signal) => {
      if (shuttingDown) return;
      log('warn', `opencode exited (code=${code} signal=${signal}) — shutting down`);
      void shutdown('child-exit');
    });
    app = buildLauncherApp(opencodeUrl);
  }

  const httpServer = serve(
    {
      fetch: app.fetch,
      port: args.port,
      hostname: '127.0.0.1',
    },
    (info) => {
      log('info', `UI listening on http://localhost:${info.port}`);
      if (opencodeUrl) {
        log('info', `API proxied at  http://localhost:${info.port}/api/* → ${opencodeUrl}`);
      }
    },
  );

  if (opencodeUrl && (args.mode === 'local' || args.mode === 'remote')) {
    attachWsUpgradeProxy(httpServer, opencodeUrl);
  }

  // -------------------------------------------------------------------------
  // Graceful shutdown
  // -------------------------------------------------------------------------
  // Two known traps we have to handle:
  //   1. @hono/node-server returns a plain http.Server. Its .close() waits
  //      for ALL active connections (including SSE long-polls) to drain,
  //      which means it hangs forever. Fix: after a short grace period
  //      call closeAllConnections() (Node 19+) / closeIdleConnections().
  //   2. opencode does not reliably react to SIGTERM on its root pid
  //      (sst/opencode#20899). We send the signal to the entire process
  //      group via stopOpencodeProcess() and SIGKILL after a grace.
  // -------------------------------------------------------------------------
  const shutdown = async (signal) => {
    if (shuttingDown) return;
    shuttingDown = true;
    log('info', `Received ${signal}, shutting down...`);

    // 1. Stop accepting new connections; force-close stragglers shortly.
    const httpClosed = new Promise((resolve) => {
      try {
        httpServer.close(() => resolve('closed'));
      } catch {
        resolve('error');
      }
    });
    setTimeout(() => {
      if (typeof httpServer.closeIdleConnections === 'function') {
        httpServer.closeIdleConnections();
      }
    }, 300);
    setTimeout(() => {
      if (typeof httpServer.closeAllConnections === 'function') {
        log('info', 'Forcing close of remaining HTTP connections');
        httpServer.closeAllConnections();
      }
    }, 1200);

    // 2. Group-kill opencode and wait for the tree to actually exit.
    let opencodeStopped = Promise.resolve('done');
    if (opencodeChild) {
      stopOpencodeProcess(opencodeChild, 'SIGTERM');
      opencodeStopped = (async () => {
        const result = await waitForChildExit(opencodeChild, 3000);
        if (result === 'timeout') {
          log('warn', 'opencode did not exit on SIGTERM, sending SIGKILL');
          stopOpencodeProcess(opencodeChild, 'SIGKILL');
          await waitForChildExit(opencodeChild, 1000);
        }
      })();
    }

    const httpResult = await raceTimeout(httpClosed, 3000);
    const ocResult = await raceTimeout(opencodeStopped, 5000);
    if (httpResult === 'timeout') log('warn', 'HTTP server close timed out');
    if (ocResult === 'timeout') log('warn', 'opencode shutdown timed out');

    process.exit(0);
  };
  process.on('SIGINT', () => void shutdown('SIGINT'));
  process.on('SIGTERM', () => void shutdown('SIGTERM'));
  process.on('SIGHUP', () => void shutdown('SIGHUP'));
}

main().catch((err) => {
  log('error', err?.stack ?? String(err));
  process.exit(1);
});
