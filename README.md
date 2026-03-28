# Bastion 🛡️

**Run AI agents safely on macOS. No Docker. No VMs. Just a 5MB binary.**

Bastion is a native macOS sandbox that gives your AI agents a safe place to execute commands — with full audit logging, network control, and rate limiting. Built in Rust. Zero dependencies at runtime.

```bash
curl -sSL https://github.com/erscoder/bastion/raw/main/install.sh | sudo bash
```

---

## The problem

You're running AI agents (Claude Code, OpenAI Codex, OpenClaw, LangChain...) and they need to execute shell commands. You have two choices:

- **Trust them blindly** — and hope they don't delete something important
- **Run Docker** — 100MB+ overhead, VM on Apple Silicon, constant friction

There's a third option.

---

## What Bastion does

Bastion runs as a lightweight HTTP server on your Mac. Every command your agents execute goes through Bastion — sandboxed, logged, and rate-limited.

```
Your AI agent
     │
     ▼
POST /api/exec {"command": "npm install"}
     │
     ▼
Bastion sandboxes it with macOS Seatbelt
     │
     ▼
{"exit_code": 0, "stdout": "...", "duration_ms": 420}
```

Your agent gets the output. You get peace of mind.

---

## Features

**🔒 Process isolation** — Uses macOS Seatbelt (sandbox-exec) at the kernel level. Agents can't touch what they're not supposed to.

**📡 Network control** — Block or allow domains. No more agents calling home to unknown endpoints.

**📋 Full audit log** — Every command, every exit code, every agent. Know exactly what ran and when.

**⏱️ Rate limiting** — Set command budgets per agent. Agents can't loop forever burning your resources.

**🔌 MCP built-in** — Native Model Context Protocol server. Plug into any MCP-compatible client in one command.

**⚡ Fast** — Rust binary, zero cold start, ~5MB on disk. Not 500MB of Docker images.

---

## Quick start

### Install

```bash
curl -sSL https://github.com/erscoder/bastion/raw/main/install.sh | sudo bash
```

### Verify it's running

```bash
curl -u bastion:bastion localhost:7575/api/health
```

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 5,
  "checks": {
    "sandbox": "ok",
    "budget": "ok"
  }
}
```

### Run your first sandboxed command

```bash
curl -u bastion:bastion -X POST localhost:7575/api/exec \
  -H "Content-Type: application/json" \
  -d '{"command": "echo hello from the sandbox"}'
```

```json
{
  "exit_code": 0,
  "stdout": "hello from the sandbox\n",
  "stderr": "",
  "duration_ms": 8
}
```

---

## Integrations

### OpenClaw

```bash
curl -fsSL https://github.com/erscoder/bastion/raw/main/bastion-setup-openclaw.sh | bash
```

From that point on, every command your OpenClaw agents run is sandboxed, audited, and rate-limited.

### Claude Code

Claude Code supports MCP servers natively. Start Bastion in MCP mode and add it to your Claude Code config:

```bash
# Start Bastion MCP server
bastion --mcp
```

Add to your Claude Code `settings.json` (usually `~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "bastion": {
      "command": "bastion",
      "args": ["--mcp"]
    }
  }
}
```

Available tools in Claude Code:
- `bastion_exec` — Run a command in the macOS sandbox
- `bastion_list_agents` — List active agents
- `bastion_get_audit` — Get execution audit logs
- `bastion_get_budget` — Check rate limit status
- `bastion_network_status` — View network rules

### OpenAI Codex CLI

Codex CLI also supports MCP. Add Bastion to your Codex config (`~/.codex/config.toml`):

```toml
[[mcp_servers]]
name = "bastion"
command = "bastion"
args = ["--mcp"]
```

Or set it via environment variable before running Codex:

```bash
export CODEX_MCP_SERVERS='[{"name":"bastion","command":"bastion","args":["--mcp"]}]'
codex
```

Every shell command Codex executes will route through Bastion's sandbox automatically.

### Any MCP client

Bastion implements the standard [Model Context Protocol](https://modelcontextprotocol.io). Any MCP-compatible client can connect:

```bash
# Start MCP server (reads JSON-RPC from stdin, writes to stdout)
bastion --mcp
```

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {"name": "bastion_exec", "description": "Execute a command in the macOS sandbox"},
      {"name": "bastion_list_agents", "description": "List active agents"},
      {"name": "bastion_get_audit", "description": "Get audit logs"},
      {"name": "bastion_get_budget", "description": "Check budget status"},
      {"name": "bastion_network_status", "description": "Network rules and stats"}
    ]
  }
}
```

---

## Sandbox profiles

Three profiles ship out of the box:

| Profile | Use case |
|---------|----------|
| `default` | Balanced. Full read access, writes to `/tmp`, network allowed |
| `strict` | Maximum isolation. No network, restricted writes, no subprocess |
| `permissive` | Development. Relaxed rules for debugging |

```bash
# Run in strict mode
curl -u bastion:bastion -X POST localhost:7575/api/exec \
  -H "Content-Type: application/json" \
  -d '{"command": "cat /etc/passwd", "profile": "strict"}'
```

Create your own `.sb` profiles in `~/.bastion/profiles/`.

---

## Network control

Block a domain from all agent traffic:

```bash
curl -u bastion:bastion -X POST localhost:7575/api/network/block \
  -H "Content-Type: application/json" \
  -d '{"domain": "suspicious-endpoint.com"}'
```

See what's running:

```bash
curl -u bastion:bastion localhost:7575/api/network/status
```

---

## Audit logs

See everything that ran:

```bash
curl -u bastion:bastion 'localhost:7575/api/audit?limit=20'
```

```json
{
  "logs": [
    {
      "timestamp": "2026-03-28T00:10:00Z",
      "agent_id": "claude-code",
      "command": "npm install",
      "exit_code": 0,
      "duration_ms": 4200,
      "profile": "default"
    }
  ],
  "total": 47
}
```

---

## Rate limiting

Each agent gets a command budget. When it runs out, Bastion returns `429 Too Many Requests`.

```bash
# Check remaining budget
curl -u bastion:bastion localhost:7575/api/budget

# Reset budget
curl -u bastion:bastion -X POST localhost:7575/api/budget/reset

# Per-agent budget
curl -u bastion:bastion localhost:7575/api/budget/claude-code
```

---

## API reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/health` | Health + dependency checks |
| `GET` | `/api/metrics` | Uptime, commands, agents |
| `GET` | `/api/agents` | List active agents |
| `DELETE` | `/api/agents/:id` | Kill an agent |
| `POST` | `/api/agents/:id/reset` | Reset agent state |
| `POST` | `/api/exec` | Execute a command in sandbox |
| `GET` | `/api/audit` | Audit logs (paginated) |
| `GET` | `/api/sandbox` | List sandbox profiles |
| `GET` | `/api/profile` | Current profile config |
| `POST` | `/api/policy/reload` | Reload profiles from disk |
| `GET` | `/api/budget` | Budget status |
| `POST` | `/api/budget/reset` | Reset budget |
| `GET` | `/api/budget/:agent_id` | Per-agent budget |
| `GET` | `/api/network/status` | Network rules + stats |
| `POST` | `/api/network/block` | Block a domain |
| `POST` | `/api/network/allow` | Allow a domain |
| `GET` | `/api/network/logs` | Network request logs |

All endpoints require HTTP Basic Auth. Default: `bastion:bastion`.

---

## Configuration

`~/.bastion/config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 7575

[auth]
username = "bastion"
password = "change-me"

[sandbox]
default_profile = "default"
profiles_dir = "/usr/local/etc/bastion/profiles"

[budget]
max_commands_per_hour = 100
max_concurrent_agents = 10
```

---

## Build from source

```bash
git clone https://github.com/erscoder/bastion.git
cd bastion
cargo build --release
./target/release/bastion          # HTTP mode on :7575
./target/release/bastion --mcp   # MCP mode (stdin/stdout)
```

Requires Rust 1.75+.

---

## License

MIT — do whatever you want with it.
