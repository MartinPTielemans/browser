#!/usr/bin/env node
// Connects an MCP client on stdio (Claude Code, Codex, ...) to Arcish's agent socket,
// starting Arcish first if it isn't running.
//
//   claude mcp add arcish -- node /path/to/arcish/tools/mcp-bridge.js
//
// Set ARCISH_USER_DATA to reach an Arcish started with the same variable.
const net = require('node:net');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');

const appDir = path.join(__dirname, '..');

function userData() {
  if (process.env.ARCISH_USER_DATA) return process.env.ARCISH_USER_DATA;
  if (process.platform === 'darwin') return path.join(os.homedir(), 'Library', 'Application Support', 'Arcish');
  if (process.platform === 'win32') return path.join(process.env.APPDATA, 'Arcish');
  return path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), '.config'), 'Arcish');
}

const sock = process.platform === 'win32' ? `\\\\.\\pipe\\arcish-agent-${os.userInfo().username}` : path.join(userData(), 'agent.sock');

const connect = () =>
  new Promise((resolve) => {
    const c = net.connect(sock);
    c.once('connect', () => resolve(c));
    c.once('error', () => resolve(null));
  });

async function main() {
  let conn = await connect();
  if (!conn) {
    const env = { ...process.env };
    delete env.ELECTRON_RUN_AS_NODE;
    spawn(require(path.join(appDir, 'node_modules', 'electron')), [appDir], { cwd: appDir, env, detached: true, stdio: 'ignore' }).unref();
    for (let i = 0; i < 80 && !conn; i++) {
      await new Promise((r) => setTimeout(r, 250));
      conn = await connect();
    }
  }
  if (!conn) {
    process.stderr.write(`Couldn't reach Arcish at ${sock}\n`);
    process.exit(1);
  }
  process.stdin.pipe(conn);
  conn.pipe(process.stdout);
  conn.on('close', () => process.exit(0));
}

main();
