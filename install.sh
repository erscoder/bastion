#!/bin/bash
#
# Bastion Installation Script — v0.1.0
# Usage: curl -sSL https://github.com/erscoder/bastion/raw/main/install.sh | sudo bash
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BINARY_NAME="bastion"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/usr/local/etc/bastion"
PROFILES_DIR="${CONFIG_DIR}/profiles"
LOG_DIR="/var/log/bastion"
LAUNCH_AGENT_DIR="${HOME}/Library/LaunchAgents"
LAUNCH_AGENT_NAME="sh.bastion.agent"
SOURCE_DIR=""  # set during build_binary

# Detect architecture
detect_architecture() {
    local arch
    arch=$(uname -m)
    case "${arch}" in
        x86_64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "arm64"
            ;;
        *)
            echo "Unsupported architecture: ${arch}" >&2
            exit 1
            ;;
    esac
}

# Detect OS
detect_os() {
    if [[ "$(uname)" != "Darwin" ]]; then
        echo "This script is for macOS only" >&2
        exit 1
    fi
}

# Check if running as root or with sudo
check_privileges() {
    if [[ $EUID -ne 0 ]]; then
        if ! sudo -v 2>/dev/null; then
            echo -e "${RED}Error: This script requires sudo privileges${NC}" >&2
            exit 1
        fi
    fi
}

# Create directories
create_directories() {
    echo -e "${YELLOW}Creating directories...${NC}"
    
    sudo mkdir -p "${INSTALL_DIR}"
    sudo mkdir -p "${CONFIG_DIR}"
    sudo mkdir -p "${PROFILES_DIR}"
    sudo mkdir -p "${LOG_DIR}"
    
    # Create user config directory
    mkdir -p "${HOME}/.bastion"
    mkdir -p "${HOME}/.bastion/profiles"
    mkdir -p "${HOME}/.bastion/logs"
    mkdir -p "${HOME}/.bastion/data"
}

# Build from source (or download pre-built binary in the future)
build_binary() {
    echo -e "${YELLOW}Building Bastion from source...${NC}"
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Rust is not installed${NC}" >&2
        echo "Please install Rust: https://rustup.rs/" >&2
        exit 1
    fi
    
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)"

    # If Cargo.toml is present we're already in the source tree
    if [[ -f "${script_dir}/Cargo.toml" ]]; then
        SOURCE_DIR="${script_dir}"
        cd "${SOURCE_DIR}"
    else
        # Installed via curl | bash — clone the repo to a temp dir
        echo -e "${YELLOW}Cloning source from GitHub...${NC}"
        SOURCE_DIR="$(mktemp -d)"
        git clone --depth 1 https://github.com/erscoder/bastion.git "${SOURCE_DIR}"
        cd "${SOURCE_DIR}"
    fi

    cargo build --release

    # Copy binary to install location
    sudo cp "target/release/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    sudo chmod 755 "${INSTALL_DIR}/${BINARY_NAME}"

    # Copy sandbox profiles now that SOURCE_DIR is set
    echo -e "${YELLOW}Copying sandbox profiles...${NC}"
    sudo mkdir -p "${PROFILES_DIR}"
    sudo cp "${SOURCE_DIR}/sandbox"/*.sb "${PROFILES_DIR}/"
    mkdir -p "${HOME}/.bastion/profiles"
    cp "${SOURCE_DIR}/sandbox"/*.sb "${HOME}/.bastion/profiles/"
    echo -e "${GREEN}Sandbox profiles installed${NC}"
}

# Create default configuration
create_config() {
    echo -e "${YELLOW}Creating configuration...${NC}"
    
    cat > "${HOME}/.bastion/config.toml" << EOF
[server]
host = "127.0.0.1"
port = 7575

[auth]
username = "bastion"
password = "bastion"

[sandbox]
default_profile = "default"
profiles_dir = "${PROFILES_DIR}"

[proxy]
enabled = true
port = 8080

[budget]
max_commands_per_hour = 100
max_concurrent_agents = 10

[logging]
level = "info"
directory = "${HOME}/.bastion/logs"
EOF
    
    echo -e "${GREEN}Configuration created at ${HOME}/.bastion/config.toml${NC}"
}

# Create sandbox profiles — profiles are now copied inside build_binary
create_profiles() {
    : # no-op: profiles copied in build_binary after SOURCE_DIR is set
}

# Create LaunchAgent for auto-start
create_launch_agent() {
    echo -e "${YELLOW}Creating LaunchAgent...${NC}"
    
    mkdir -p "${LAUNCH_AGENT_DIR}"
    
    cat > "${LAUNCH_AGENT_DIR}/${LAUNCH_AGENT_NAME}.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${LAUNCH_AGENT_NAME}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/${BINARY_NAME}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${HOME}/.bastion/logs/bastion.log</string>
    <key>StandardErrorPath</key>
    <string>${HOME}/.bastion/logs/bastion.log</string>
</dict>
</plist>
EOF
    
    echo -e "${GREEN}LaunchAgent created at ${LAUNCH_AGENT_DIR}/${LAUNCH_AGENT_NAME}.plist${NC}"
}

# Verify installation
verify_installation() {
    echo -e "${YELLOW}Verifying installation...${NC}"
    
    # Check binary exists
    if [[ ! -f "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        echo -e "${RED}Error: Binary not found at ${INSTALL_DIR}/${BINARY_NAME}${NC}" >&2
        exit 1
    fi
    
    # Check binary is executable
    if [[ ! -x "${INSTALL_DIR}/${BINARY_NAME}" ]]; then
        echo -e "${RED}Error: Binary not executable${NC}" >&2
        exit 1
    fi
    
    # Check configuration exists
    if [[ ! -f "${HOME}/.bastion/config.toml" ]]; then
        echo -e "${RED}Error: Configuration not found${NC}" >&2
        exit 1
    fi
    
    # Check profiles exist
    if [[ ! -f "${PROFILES_DIR}/default.sb" ]]; then
        echo -e "${RED}Error: Default profile not found${NC}" >&2
        exit 1
    fi
    
    # Start the service
    echo -e "${YELLOW}Starting Bastion service...${NC}"
    launchctl load "${LAUNCH_AGENT_DIR}/${LAUNCH_AGENT_NAME}.plist" 2>/dev/null || true
    
    # Wait for service to start
    sleep 2
    
    # Verify service is running
    if curl -s -u bastion:bastion "http://localhost:7575/api/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Bastion is running${NC}"
        
        # Get version
        local version
        version=$(curl -s -u bastion:bastion "http://localhost:7575/api/health" | grep -o '"version":"[^"]*"' | cut -d'"' -f4)
        echo -e "${GREEN}✓ Version: ${version}${NC}"
    else
        echo -e "${RED}Warning: Could not verify service (may need to start manually)${NC}"
    fi
    
    echo ""
    echo -e "${GREEN}Installation complete!${NC}"
    echo ""
    echo "To start Bastion manually:"
    echo "  sudo ${BINARY_NAME} &"
    echo ""
    echo "To verify:"
    echo "  curl -u bastion:bastion localhost:7575/api/health"
    echo ""
    echo "Default credentials: bastion:bastion"
}

# Main
main() {
    echo "Bastion Installer"
    echo "================="
    echo ""
    
    detect_os
    check_privileges
    
    local arch
    arch=$(detect_architecture)
    echo -e "Architecture: ${GREEN}${arch}${NC}"
    
    create_directories
    create_config
    build_binary       # sets SOURCE_DIR — must run before create_profiles
    create_profiles
    create_launch_agent
    verify_installation
}

main "$@"