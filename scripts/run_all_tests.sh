#!/usr/bin/env bash
# ==============================================================================
# Multiplay Memory-Efficient Test Suite Runner
# Executes test suites crate-by-crate with bounded parallelism to prevent OOM
# ==============================================================================
set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BOLD}${BLUE}================================================================${NC}"
echo -e "${BOLD}${BLUE}   Multiplay Pure-Rust Test Suite (Memory-Bounded Runner)       ${NC}"
echo -e "${BOLD}${BLUE}================================================================${NC}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

CRATES=(
    "amp-domain"
    "amp-ports"
    "amp-core-engine"
    "amp-adapter-decoder"
    "amp-adapter-cpal"
    "amp-adapter-storage"
    "amp-adapter-soulseek"
    "amp-adapter-p2p"
    "amp-adapter-youtube"
    "amp-adapter-tui"
    "amp-adapter-gui"
    "amp-app"
)

TOTAL=${#CRATES[@]}
PASSED=0
FAILED=0
START_TIME=$(date +%s)

for idx in "${!CRATES[@]}"; do
    crate="${CRATES[$idx]}"
    num=$((idx + 1))
    echo -e "\n${BOLD}[${num}/${TOTAL}] Testing crate: ${crate}...${NC}"

    # Use bounded 2 test threads to guarantee low peak RSS memory footprint
    if cargo test -p "${crate}" -- --test-threads=2; then
        echo -e "${GREEN}✓ ${crate} passed${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗ ${crate} failed${NC}"
        FAILED=$((FAILED + 1))
        exit 1
    fi
done

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo -e "\n${BOLD}${BLUE}================================================================${NC}"
echo -e "${BOLD}${GREEN}All ${PASSED}/${TOTAL} workspace crates passed in ${DURATION}s with zero crashes.${NC}"
echo -e "${BOLD}${BLUE}================================================================${NC}"
exit 0
