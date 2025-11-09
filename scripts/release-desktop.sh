#!/usr/bin/env bash

# Automates the release flow for the cpt.run desktop app.
# - Verifies version metadata and changelog entries.
# - Runs fmt + test to ensure the tree is green.
# - Triggers the GitHub Actions desktop-release workflow via the GitHub CLI.

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "The GitHub CLI (gh) must be installed and authenticated." >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "GitHub CLI is not authenticated. Run 'gh auth login' first." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Cargo (Rust toolchain) is required to build the desktop release." >&2
  exit 1
fi

VERSION="$1"
TAG="desktop-v${VERSION}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

CARGO_MANIFEST="${REPO_ROOT}/desktop/Cargo.toml"
CHANGELOG="${REPO_ROOT}/docs/CHANGELOG.md"

if [[ ! -f "${CARGO_MANIFEST}" ]]; then
  echo "Unable to locate desktop/Cargo.toml at ${CARGO_MANIFEST}" >&2
  exit 1
fi

CURRENT_VERSION="$(sed -n 's/^version = \"\(.*\)\"/\1/p' "${CARGO_MANIFEST}" | head -n 1)"
if [[ "${CURRENT_VERSION}" != "${VERSION}" ]]; then
  echo "Version mismatch: requested ${VERSION}, but Cargo.toml reports ${CURRENT_VERSION}." >&2
  exit 1
fi

if [[ ! -f "${CHANGELOG}" ]]; then
  echo "Unable to locate changelog at ${CHANGELOG}" >&2
  exit 1
fi

if ! grep -q "## \\[${VERSION}\\]" "${CHANGELOG}"; then
  echo "Changelog entry for ${VERSION} not found. Add a Keep a Changelog section before releasing." >&2
  exit 1
fi

echo "Running cargo fmt + cargo test before triggering the release workflow..."
(
  cd "${REPO_ROOT}/desktop"
  cargo fmt --all
  cargo test --workspace
)

echo "Triggering GitHub Actions workflow for ${TAG}..."
DEFAULT_BRANCH="$(git -C "${REPO_ROOT}" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "main")"
if [[ "${DEFAULT_BRANCH}" == "HEAD" ]]; then
  DEFAULT_BRANCH="main"
fi

REPO_NAME="$(gh repo view --json nameWithOwner -q '.nameWithOwner')"

gh workflow run desktop-release.yml \
  --repo "${REPO_NAME}" \
  --ref "${DEFAULT_BRANCH}" \
  --field version="${VERSION}"

echo "Release workflow dispatched. Monitor progress:"
echo "  gh run watch --workflow desktop-release.yml"
echo "When the workflow finishes, run scripts/homebrew/update_formula.sh ${VERSION} to refresh the tap."
