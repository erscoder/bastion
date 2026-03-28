# Bastion API Documentation

## Base URL

```
http://localhost:7575
```

## Authentication

All endpoints (except `/api/health`) require HTTP Basic Authentication.

**Default credentials:** `bastion:bastion`

```bash
curl -u bastion:bastion http://localhost:7575/api/...
```

---

## Endpoints

### 1. Health Check

**GET** `/api/health`

Returns the health status of the Bastion server.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 42
}
```

**No authentication required.**

---

### 2. List Agents

**GET** `/api/agents`

Returns a list of all active agents.

**Response:**
```json
{
  "agents": [
    {
      "id": "agent-abc123",
      "name": "claude-code",
      "created_at": "2026-03-27T10:30:00Z",
      "commands_executed": 15,
      "status": "running"
    }
  ]
}
```

---

### 3. Execute Command

**POST** `/api/exec`

Executes a command in the sandbox.

**Request Body:**
```json
{
  "command": "ls -la",
  "profile": "default",
  "agent_id": "agent-abc123",
  "timeout_ms": 30000
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `command` | string | Yes | Command to execute |
| `profile` | string | No | Sandbox profile (default, strict, permissive) |
| `agent_id` | string | No | Agent ID for tracking |
| `timeout_ms` | integer | No | Timeout in milliseconds (default: 30000) |

**Response:**
```json
{
  "exit_code": 0,
  "stdout": "total 0\ndrwxr-xr-x 1",
  "stderr": "",
  "duration_ms": 25,
  "agent_id": "agent-abc123"
}
```

---

### 4. Audit Logs

**GET** `/api/audit`

Returns audit logs of all command executions.

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 100 | Maximum number of logs |
| `offset` | integer | 0 | Offset for pagination |

**Response:**
```json
{
  "logs": [
    {
      "timestamp": "2026-03-27T10:30:00Z",
      "agent_id": "agent-abc123",
      "command": "ls -la",
      "exit_code": 0,
      "duration_ms": 25,
      "profile": "default"
    }
  ],
  "total": 1
}
```

---

### 5. Sandbox Profiles

**GET** `/api/sandbox`

Returns a list of available sandbox profiles.

**Response:**
```json
{
  "profiles": ["default", "strict", "permissive"],
  "active": "default"
}
```

---

### 6. Profile Configuration

**GET** `/api/profile`

Returns the configuration of the active sandbox profile.

**Response:**
```json
{
  "name": "default",
  "description": "Balanced security for general use",
  "filesystem": {
    "allowed_read": ["/tmp", "/var/folders"],
    "allowed_write": ["/tmp"],
    "denied": ["/System", "/Users/*/Library"]
  },
  "network": {
    "proxy_enabled": true,
    "allowed_domains": ["*.openai.com", "*.anthropic.com"],
    "blocked_domains": []
  }
}
```

---

### 7. Reload Policies

**POST** `/api/policy/reload`

Reloads sandbox policies from disk.

**Response:**
```json
{
  "status": "reloaded",
  "profiles_loaded": 3
}
```

---

### 8. Budget Status

**GET** `/api/budget`

Returns budget and rate limiting status.

**Response:**
```json
{
  "agents": 1,
  "total_commands": 15,
  "rate_limit": 100,
  "commands_remaining": 85
}
```

---

### 9. Delete Agent

**DELETE** `/api/agents/{id}`

Kills an active agent.

**Response:**
```json
{
  "status": "killed",
  "agent_id": "agent-abc123"
}
```

---

### 10. Reset Agent

**POST** `/api/agents/{id}/reset`

Resets an agent's state (clears command count, etc.).

**Response:**
```json
{
  "status": "reset",
  "agent_id": "agent-abc123"
}
```

---

## Error Responses

All endpoints may return error responses:

**401 Unauthorized:**
```json
{
  "error": "Unauthorized",
  "message": "Invalid or missing credentials"
}
```

**404 Not Found:**
```json
{
  "error": "Not Found",
  "message": "Agent not found"
}
```

**500 Internal Server Error:**
```json
{
  "error": "Internal Server Error",
  "message": "Command execution failed"
}
```

---

## Rate Limiting

- Default rate limit: 100 commands per hour per agent
- Configurable in `~/.bastion/config.toml`
- Returns `429 Too Many Requests` when limit exceeded

---

## Python SDK Example

```python
import requests
from requests.auth import HTTPBasicAuth

class BastionClient:
    def __init__(self, host="localhost", port=7575, username="bastion", password="bastion"):
        self.base_url = f"http://{host}:{port}"
        self.auth = HTTPBasicAuth(username, password)
    
    def health(self):
        return requests.get(f"{self.base_url}/api/health").json()
    
    def list_agents(self):
        return requests.get(f"{self.base_url}/api/agents", auth=self.auth).json()
    
    def execute(self, command, profile="default", agent_id=None, timeout_ms=30000):
        return requests.post(
            f"{self.base_url}/api/exec",
            json={"command": command, "profile": profile, "agent_id": agent_id, "timeout_ms": timeout_ms},
            auth=self.auth
        ).json()
    
    def get_audit(self, limit=100, offset=0):
        return requests.get(
            f"{self.base_url}/api/audit",
            params={"limit": limit, "offset": offset},
            auth=self.auth
        ).json()
    
    def get_budget(self):
        return requests.get(f"{self.base_url}/api/budget", auth=self.auth).json()
    
    def delete_agent(self, agent_id):
        return requests.delete(f"{self.base_url}/api/agents/{agent_id}", auth=self.auth).json()
    
    def reset_agent(self, agent_id):
        return requests.post(f"{self.base_url}/api/agents/{agent_id}/reset", auth=self.auth).json()

# Usage
client = BastionClient()
print(client.health())
result = client.execute("echo hello", profile="default")
print(result)
```

---

## JavaScript SDK Example

```javascript
class BastionClient {
  constructor(options = {}) {
    this.baseUrl = `http://${options.host || 'localhost'}:${options.port || 7575}`;
    this.auth = btoa(`${options.username || 'bastion'}:${options.password || 'bastion'}`);
  }
  
  async request(endpoint, options = {}) {
    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers: {
        'Authorization': `Basic ${this.auth}`,
        'Content-Type': 'application/json',
        ...options.headers
      }
    });
    return response.json();
  }
  
  async health() {
    return this.request('/api/health');
  }
  
  async listAgents() {
    return this.request('/api/agents');
  }
  
  async execute(command, profile = 'default', agentId = null) {
    return this.request('/api/exec', {
      method: 'POST',
      body: JSON.stringify({ command, profile, agent_id: agentId })
    });
  }
  
  async getAudit(limit = 100, offset = 0) {
    return this.request(`/api/audit?limit=${limit}&offset=${offset}`);
  }
  
  async getBudget() {
    return this.request('/api/budget');
  }
  
  async deleteAgent(agentId) {
    return this.request(`/api/agents/${agentId}`, { method: 'DELETE' });
  }
  
  async resetAgent(agentId) {
    return this.request(`/api/agents/${agentId}/reset`, { method: 'POST' });
  }
}

// Usage
const client = new BastionClient();
console.log(await client.health());
const result = await client.execute('echo hello', 'default');
console.log(result);
```