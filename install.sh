#!/bin/bash

set -e

REPO="aliabdoxd14-sudo/packman"
INSTALL_DIR="${HOME}/.local/bin"
VERSION=$(curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | grep -o '"tag_name": "[^"]*' | cut -d'"' -f4)

detect_platform() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "darwin" ;;
        *)          echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64)     echo "x86_64" ;;
        aarch64|arm64) echo "arm64" ;;
        *)          echo "x86_64" ;;
    esac
}

install() {
    local platform=$(detect_platform)
    local arch=$(detect_arch)
    
    local filename="packman-${platform}-${arch}"
    local url="https://github.com/${REPO}/releases/download/${VERSION}/${filename}"
    
    echo "Downloading PackMan ${VERSION} for ${platform}-${arch}..."
    curl -fSL "$url" -o "/tmp/packman" || die "Failed to download PackMan"
    
    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
    fi
    
    mv "/tmp/packman" "${INSTALL_DIR}/packman"
    chmod +x "${INSTALL_DIR}/packman"
    
    echo "Installed PackMan to ${INSTALL_DIR}/packman"
    echo "Add '${INSTALL_DIR}' to your PATH to use packman from anywhere"
}

die() {
    echo "Error: $1" >&2
    exit 1
}

main() {
    install
}

main "$@"
