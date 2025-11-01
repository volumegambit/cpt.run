#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd -- "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ICON_SRC_DIR="${APP_DIR}/icons"
ICON_DEST_DIR="${APP_DIR}/crates/desktop/icons"
ICONSET_DIR="${APP_DIR}/target/macos-icon.iconset"
ICNS_PATH="${ICON_DEST_DIR}/cpt.icns"

ensure_iconutil() {
    if ! command -v iconutil >/dev/null 2>&1; then
        echo "error: macOS 'iconutil' tool is required to build the .icns icon." >&2
        echo "Install Xcode command line tools if it is missing: xcode-select --install" >&2
        exit 1
    fi
}

regenerate_icns() {
    rm -rf "${ICONSET_DIR}"
    mkdir -p "${ICONSET_DIR}"
    mkdir -p "${ICON_DEST_DIR}"

    local icons=(
        "icon_16x16.png"
        "icon_16x16@2x.png"
        "icon_32x32.png"
        "icon_32x32@2x.png"
        "icon_128x128.png"
        "icon_128x128@2x.png"
        "icon_256x256.png"
        "icon_256x256@2x.png"
        "icon_512x512.png"
        "icon_512x512@2x.png"
        "icon_1024x1024.png"
    )

    for filename in "${icons[@]}"; do
        if [[ ! -f "${ICON_SRC_DIR}/${filename}" ]]; then
            echo "error: missing required icon asset: ${ICON_SRC_DIR}/${filename}" >&2
            exit 1
        fi
        cp "${ICON_SRC_DIR}/${filename}" "${ICONSET_DIR}/${filename}"
    done

    iconutil -c icns "${ICONSET_DIR}" -o "${ICNS_PATH}"
    echo "Generated ${ICNS_PATH}"
    rm -rf "${ICONSET_DIR}"
}

build_bundle() {
    (
        cd "${APP_DIR}/crates/desktop"
        cargo bundle --release --bin cpt-desktop "$@"
    )
}

main() {
    ensure_iconutil
    regenerate_icns
    build_bundle "$@"
    echo
    echo "Bundle created under target/*/release/bundle/osx/"
}

main "$@"
