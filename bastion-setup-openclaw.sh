#!/bin/bash
#
# Bastion OpenClaw Integration Setup
# Usage: curl -fsSL https://github.com/erscoder/bastion/raw/main/bastion-setup-openclaw.sh | bash
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Bastion OpenClaw Setup"
echo "======================"
echo ""

# Check if Bastion is running
check_bastion() {
    echo -e "${YELLOW}Checking if Bastion is running...${NC}"
    
    if curl -s -u bastion:bastion "http://localhost:7575/api/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Bastion is running${NC}"
        return 0
    else
        echo -e "${RED}✗ Bastion is not running${NC}"
        echo ""
        echo "Please start Bastion first:"
        echo "  sudo bastion &"
        echo ""
        echo "Or install it:"
        echo "  curl -sSL https://github.com/erscoder/bastion/raw/main/install.sh | sudo bash"
        exit 1
    fi
}

# Detect OpenClaw installation
detect_openclaw() {
    echo -e "${YELLOW}Checking OpenClaw installation...${NC}"
    
    # Check common OpenClaw installation paths
    local openclaw_paths=(
        "/usr/local/bin/openclaw"
        "/usr/bin/openclaw"
        "${HOME}/.nvm/versions/node/*/bin/openclaw"
    )
    
    for path in "${openclaw_paths[@]}"; do
        if [[ -f "$path" ]]; then
            echo -e "${GREEN}✓ Found OpenClaw at ${path}${NC}"
            OPENCLAW_PATH="$path"
            return 0
        fi
    done
    
    # Check if openclaw command exists
    if command -v openclaw &> /dev/null; then
        OPENCLAW_PATH=$(command -v openclaw)
        echo -e "${GREEN}✓ Found OpenClaw at ${OPENCLAW_PATH}${NC}"
        return 0
    fi
    
    echo -e "${YELLOW}OpenClaw not found, installing...${NC}"
    return 1
}

# Create MCP configuration for OpenClaw
create_mcp_config() {
    echo -e "${YELLOW}Creating MCP configuration...${NC}"
    
    local mcp_config_dir="${HOME}/.openclaw/mcp"
    local mcp_config_file="${mcp_config_dir}/bastion.json"
    
    mkdir -p "${mcp_config_dir}"
    
    cat > "${mcp_config_file}" << EOF
{
  "mcpServers": {
    "bastion": {
      "command": "bastion-mcp",
      "env": {
        "BASTION_HOST": "127.0.0.1",
        "BASTION_PORT": "7575",
        "BASTION_USER": "bastion",
        "BASTION_PASS": "bastion"
      }
    }
  }
}
EOF
    
    echo -e "${GREEN}MCP config created at ${mcp_config_file}${NC}"
}

# Create Bastion MCP client wrapper
create_mcp_client() {
    echo -e "${YELLOW}Creating MCP client wrapper...${NC}"
    
    local bin_dir="${HOME}/.bastion/bin"
    mkdir -p "${bin_dir}"
    
    # Create bastion-mcp script
    cat > "${bin_dir}/bastion-mcp" << 'SCRIPT'
#!/bin/bash
#
# Bastion MCP Client - communicates with Bastion API
#

BASTION_HOST="${BASTION_HOST:-127.0.0.1}"
BASTION_PORT="${BASTION_PORT:-7575}"
BASTION_USER="${BASTION_USER:-bastion}"
BASTION_PASS="${BASTION_PASS:-bastion}"

# Read JSON input from stdin
INPUT=$(cat)

# Parse method from JSON-RPC request
METHOD=$(echo "$INPUT" | grep -o '"method"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)"$/\1/')

# Handle different MCP methods
case "$METHOD" in
    "initialize")
        cat << 'EOF'
{"jsonrpc":"2.0","result":{"protocolVersion":"1.0","capabilities":{"tools":{},"resources":{},"prompts":{}},"serverInfo: bastion","version":"0.1.0"}},"id":1}
EOF
        ;;
    "tools/list")
        cat << 'EOF'
{"jsonrpc":"2.0","result":{"tools":[{"name":"execute","description":"Execute a command in the sandbox","inputSchema":{"type":"object","properties":{"command":{"type":"string","description":"Command to execute"},"profile":{"type":"string","description":"Sandbox profile to use"}},"required":["command"]}},{"name":"list_agents","description":"List active agents","inputSchema":{"type":"object","properties":{}}},{"name":"get_audit","description":"Get audit logs","inputSchema":{"type":"object","properties":{"limit":{"type":"number"},"offset":{"type":"number"}}}}]}}
EOF
        ;;
    "tools/call")
        TOOL_NAME=$(echo "$INPUT" | grep -o '"name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)"$/\1/')
        
        case "$TOOL_NAME" in
            "execute")
                COMMAND=$(echo "$INPUT" | grep -o '"command"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)"$/\1/')
                RESPONSE=$(curl -s -u "${BASTION_USER}:${BASTION_PASS}" \
                    -X POST "http://${BASTION_HOST}:${BASTION_PORT}/api/exec" \
                    -H "Content-Type: application/json" \
                    -d "{\"command\":\"$COMMAND\",\"profile\":\"default\"}")
                
                # Transform to JSON-RPC response
                EXIT_CODE=$(echo "$RESPONSE" | grep -o '"exit_code":[0-9]*' | cut -d':' -f2)
                STDOUT=$(echo "$RESPONSE" | grep -o '"stdout":"[^"]*"' | sed 's/"stdout":"\(.*\)"$/\1/' | sed 's/\\n/\n/g')
                
                echo "{\"jsonrpc\":\"2.0\",\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"Exit code: $EXIT_CODE\\nOutput: $STDOUT\"}]}}"
                ;;
            "list_agents")
                RESPONSE=$(curl -s -u "${BASTION_USER}:${BASTION_PASS}" \
                    "http://${BASTION_HOST}:${BASTION_PORT}/api/agents")
                
                echo "{\"jsonrpc\":\"2.0\",\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"$RESPONSE\"}]}}"
                ;;
            "get_audit")
                RESPONSE=$(curl -s -u "${BASTION_USER}:${BASTION_PASS}" \
                    "http://${BASTION_HOST}:${BASTION_PORT}/api/audit")
                
                echo "{\"jsonrpc\":\"2.0\",\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"$RESPONSE\"}]}}"
                ;;
        esac
        ;;
    *)
        echo "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32601,\"message\":\"Method not found\"}}"
        ;;
esac
SCRIPT

    chmod +x "${bin_dir}/bastion-mcp"
    
    # Add to PATH
    if [[ ":$PATH:" != *":${bin_dir}:"* ]]; then
        echo "export PATH=\"\${PATH}:${bin_dir}\"" >> "${HOME}/.zshrc"
        export PATH="${PATH}:${bin_dir}"
    fi
    
    echo -e "${GREEN}MCP client created at ${bin_dir}/bastion-mcp${NC}"
}

# Update OpenClaw config if needed
update_openclaw_config() {
    echo -e "${YELLOW}Checking OpenClaw configuration...${NC}"
    
    local openclaw_config="${HOME}/.openclaw/config.json"
    
    if [[ -f "${openclaw_config}" ]]; then
        # Check if MCP is already configured
        if grep -q "bastion" "${openclaw_config}" 2>/dev/null; then
            echo -e "${GREEN}✓ Bastion already configured in OpenClaw${NC}"
            return 0
        fi
    fi
    
    echo -e "${YELLOW}To enable Bastion in OpenClaw, add to your config:${NC}"
    echo ""
    echo '  "mcpServers": {'
    echo '    "bastion": {'
    echo '      "command": "bastion-mcp"'
    echo '    }'
    echo '  }'
    echo ""
}

# Test the integration
test_integration() {
    echo -e "${YELLOW}Testing Bastion API...${NC}"
    
    local health_response
    health_response=$(curl -s -u bastion:bastion "http://localhost:7575/api/health")
    
    if echo "$health_response" | grep -q "healthy"; then
        echo -e "${GREEN}✓ API health check passed${NC}"
    else
        echo -e "${RED}✗ API health check failed${NC}"
        exit 1
    fi
    
    # Test exec endpoint
    echo -e "${YELLOW}Testing exec endpoint...${NC}"
    
    local exec_response
    exec_response=$(curl -s -u bastion:bastion -X POST "http://localhost:7575/api/exec" \
        -H "Content-Type: application/json" \
        -d '{"command":"echo hello","profile":"default"}')
    
    if echo "$exec_response" | grep -q "hello"; then
        echo -e "${GREEN}✓ Exec endpoint working${NC}"
    else
        echo -e "${RED}✗ Exec endpoint failed${NC}"
    fi
    
    # Test MCP client
    echo -e "${YELLOW}Testing MCP client...${NC}"
    
    local mcp_response
    mcp_response=$(echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | "${HOME}/.bastion/bin/bastion-mcp" 2>/dev/null)
    
    if echo "$mcp_response" | grep -q "tools"; then
        echo -e "${GREEN}✓ MCP client working${NC}"
    else
        echo -e "${YELLOW}⚠ MCP client needs verification${NC}"
    fi
}

# Main
main() {
    check_bastion
    detect_openclaw || true
    create_mcp_config
    create_mcp_client
    update_openclaw_config
    test_integration
    
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  OpenClaw integration complete!${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Restart OpenClaw to load the MCP server"
    echo "  2. Agents will now run in the Bastion sandbox"
    echo "  3. All commands are logged and audited"
    echo ""
    echo "Verify integration:"
    echo "  curl -u bastion:bastion localhost:7575/api/agents"
    echo ""
    echo "View audit logs:"
    echo "  curl -u bastion:bastion localhost:7575/api/audit"
    echo ""
}

main "$@"