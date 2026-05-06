#!/bin/sh
# shellcheck shell=sh
#
# Arkouda install script
# Run with: curl -sSfL https://raw.githubusercontent.com/manuelmauro/arkouda/main/install.sh | sh
#
# This script installs arkouda to ~/.cargo/bin (if cargo is available) or ~/.local/bin
# It will attempt to download a pre-built binary first, falling back to cargo install

set -e

REPO="manuelmauro/arkouda"
BINARY_NAME="arkouda"

# Colors for output (only if terminal supports it)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    NC=''
fi

info() {
    printf "${BLUE}info:${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}success:${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn:${NC} %s\n" "$1"
}

error() {
    printf "${RED}error:${NC} %s\n" "$1" >&2
}

# Detect target triple (matches Rust target naming)
# Supported pre-built binaries: Linux x86_64, macOS ARM, Windows x86_64
# Other platforms will fall back to cargo install
detect_target() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux)
            case "$arch" in
                x86_64|amd64)   echo "x86_64-unknown-linux-gnu" ;;
                *)              echo "unknown" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                aarch64|arm64)  echo "aarch64-apple-darwin" ;;
                *)              echo "unknown" ;;
            esac
            ;;
        CYGWIN*|MINGW*|MSYS*|Windows_NT)
            case "$arch" in
                x86_64|amd64)   echo "x86_64-pc-windows-msvc" ;;
                *)              echo "unknown" ;;
            esac
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Get archive extension for platform
get_archive_ext() {
    case "$1" in
        *windows*) echo "zip" ;;
        *)         echo "tar.gz" ;;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    if command -v curl > /dev/null 2>&1; then
        curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | \
            grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo ""
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | \
            grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo ""
    else
        echo ""
    fi
}

# Check if pre-built binary exists for this platform
check_binary_available() {
    version="$1"
    target="$2"

    ext=$(get_archive_ext "$target")
    asset_name="${BINARY_NAME}-${target}.${ext}"

    if command -v curl > /dev/null 2>&1; then
        status=$(curl -sSfL -o /dev/null -w "%{http_code}" \
            "https://github.com/${REPO}/releases/download/${version}/${asset_name}" 2>/dev/null || echo "404")
    elif command -v wget > /dev/null 2>&1; then
        if wget -q --spider "https://github.com/${REPO}/releases/download/${version}/${asset_name}" 2>/dev/null; then
            status="200"
        else
            status="404"
        fi
    else
        status="404"
    fi

    if [ "$status" = "200" ]; then
        echo "${asset_name}"
    else
        echo ""
    fi
}

# Download and install pre-built binary
install_binary() {
    version="$1"
    asset="$2"
    install_dir="$3"

    info "Downloading ${BINARY_NAME} ${version}..."

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    download_url="https://github.com/${REPO}/releases/download/${version}/${asset}"

    if command -v curl > /dev/null 2>&1; then
        curl -sSfL "$download_url" -o "${tmp_dir}/${asset}"
    elif command -v wget > /dev/null 2>&1; then
        wget -q "$download_url" -O "${tmp_dir}/${asset}"
    fi

    info "Extracting..."

    case "$asset" in
        *.tar.gz)
            tar -xzf "${tmp_dir}/${asset}" -C "$tmp_dir"
            ;;
        *.zip)
            unzip -q "${tmp_dir}/${asset}" -d "$tmp_dir"
            ;;
    esac

    # Find the binary (handles both arkouda and arkouda.exe)
    binary_path=$(find "$tmp_dir" -name "$BINARY_NAME" -o -name "${BINARY_NAME}.exe" 2>/dev/null | head -1)

    if [ -z "$binary_path" ]; then
        error "Could not find ${BINARY_NAME} binary in archive"
        return 1
    fi

    mkdir -p "$install_dir"
    chmod +x "$binary_path"

    # Preserve .exe extension on Windows
    binary_dest="${install_dir}/${BINARY_NAME}"
    case "$binary_path" in
        *.exe) binary_dest="${binary_dest}.exe" ;;
    esac
    mv "$binary_path" "$binary_dest"

    return 0
}

# Install via cargo
install_via_cargo() {
    if ! command -v cargo > /dev/null 2>&1; then
        return 1
    fi

    info "Installing via cargo..."
    cargo install arkouda
    return 0
}

# Get the installation directory
get_install_dir() {
    # Prefer cargo bin if it exists
    if [ -n "$CARGO_HOME" ]; then
        echo "${CARGO_HOME}/bin"
    elif [ -d "$HOME/.cargo/bin" ]; then
        echo "$HOME/.cargo/bin"
    else
        echo "$HOME/.local/bin"
    fi
}

# Check if a directory is in PATH
in_path() {
    case ":$PATH:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

# Detect shell config file
detect_shell_config() {
    shell_name=$(basename "${SHELL:-sh}")

    case "$shell_name" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                echo "$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                echo "$HOME/.bash_profile"
            else
                echo "$HOME/.profile"
            fi
            ;;
        zsh)
            echo "$HOME/.zshrc"
            ;;
        fish)
            echo "$HOME/.config/fish/config.fish"
            ;;
        *)
            echo "$HOME/.profile"
            ;;
    esac
}

main() {
    printf "\n"
    printf "${BOLD}Arkouda Installer${NC}\n"
    printf "==================\n\n"

    target=$(detect_target)

    info "Detected target: ${target}"

    if [ "$target" = "unknown" ]; then
        warn "Could not detect platform, will try cargo install"
    fi

    install_dir=$(get_install_dir)
    info "Install directory: ${install_dir}"

    # Try to get latest version and check for pre-built binary
    version=$(get_latest_version)
    installed_via="binary"

    if [ -n "$version" ] && [ "$target" != "unknown" ]; then
        info "Latest version: ${version}"

        asset=$(check_binary_available "$version" "$target")

        if [ -n "$asset" ]; then
            info "Pre-built binary available: ${asset}"
            if install_binary "$version" "$asset" "$install_dir"; then
                success "Installed ${BINARY_NAME} ${version} to ${install_dir}"
            else
                warn "Binary installation failed, trying cargo..."
                installed_via="cargo"
                if ! install_via_cargo; then
                    error "Could not install via cargo"
                    error "Please install Rust first: https://rustup.rs"
                    exit 1
                fi
            fi
        else
            info "No pre-built binary for ${target}, using cargo..."
            installed_via="cargo"
            if ! install_via_cargo; then
                error "Cargo not found and no pre-built binary available"
                error "Please install Rust first: https://rustup.rs"
                exit 1
            fi
        fi
    else
        info "Could not fetch release info, using cargo..."
        installed_via="cargo"
        if ! install_via_cargo; then
            error "Cargo not found and could not fetch release info"
            error "Please install Rust first: https://rustup.rs"
            exit 1
        fi
    fi

    # Check if install directory is in PATH
    if [ "$installed_via" = "binary" ] && ! in_path "$install_dir"; then
        printf "\n"
        warn "${install_dir} is not in your PATH"

        shell_config=$(detect_shell_config)
        shell_name=$(basename "${SHELL:-sh}")

        printf "\n"
        info "Add it to your PATH by running:"
        printf "\n"

        if [ "$shell_name" = "fish" ]; then
            printf "    ${BOLD}fish_add_path %s${NC}\n" "$install_dir"
        else
            printf "    ${BOLD}echo 'export PATH=\"%s:\$PATH\"' >> %s${NC}\n" "$install_dir" "$shell_config"
        fi

        printf "\n"
        info "Then reload your shell or run:"
        printf "\n"

        if [ "$shell_name" = "fish" ]; then
            printf "    ${BOLD}source %s${NC}\n" "$shell_config"
        else
            printf "    ${BOLD}source %s${NC}\n" "$shell_config"
        fi
    fi

    printf "\n"
    success "${BINARY_NAME} installation complete!"
    printf "\n"
    info "Get started with:"
    printf "    ${BOLD}arkouda --help${NC}\n"
    printf "\n"
}

main "$@"
