#!/bin/bash

# Parity test runner - compares Python Click output with Rust click-rs output

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PHASE="${1:-phase1}"
MODULE="${2:-all}"
PINNED_CLICK_VERSION="8.3.1"

# Use a dedicated venv so parity stays pinned to the Click version we ported against.
PARITY_PYTHON="${PARITY_PYTHON:-python3}"
VENV_DIR="${SCRIPT_DIR}/.venv"
PARITY_ALLOW_CLICK_SRC="${PARITY_ALLOW_CLICK_SRC:-0}"

ensure_venv() {
    if [ ! -x "${VENV_DIR}/bin/python" ]; then
        echo -e "${CYAN}Creating parity venv...${NC}"
        "${PARITY_PYTHON}" -m venv "${VENV_DIR}"
    fi

    # shellcheck disable=SC1090
    PYTHON="${VENV_DIR}/bin/python"

    local installed_click
    installed_click="$("${PYTHON}" -c 'import importlib.metadata as m; print(m.version("click"))' 2>/dev/null || true)"

    if [ "${installed_click}" != "${PINNED_CLICK_VERSION}" ]; then
        echo -e "${CYAN}Installing pinned Click ${PINNED_CLICK_VERSION} into parity venv...${NC}"
        "${PYTHON}" -m pip -q install --upgrade pip >/dev/null
        "${PYTHON}" -m pip -q install -r "${SCRIPT_DIR}/requirements.txt"
    fi

    installed_click="$("${PYTHON}" -c 'import importlib.metadata as m; print(m.version("click"))')"
    if [ "${installed_click}" != "${PINNED_CLICK_VERSION}" ]; then
        echo -e "${RED}Error: parity venv Click version mismatch (have ${installed_click}, want ${PINNED_CLICK_VERSION})${NC}"
        exit 1
    fi
}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

ensure_venv

PHASE_DIR="$SCRIPT_DIR/$PHASE"

if [ ! -d "$PHASE_DIR" ]; then
    echo -e "${RED}Error: Phase directory not found: $PHASE_DIR${NC}"
    exit 1
fi

# Build Rust crate once
echo -e "${CYAN}Building Rust parity crate...${NC}"
(cd "$PHASE_DIR/rust" && cargo build --release --quiet)
RUST_BIN="$PHASE_DIR/rust/target/release/parity-${PHASE}"
chmod +x "$RUST_BIN" 2>/dev/null || true

normalize_phase2_output() {
    local infile="$1"
    local outfile="$2"
    python3 - "$infile" "$outfile" <<'PY'
import re
import sys

infile, outfile = sys.argv[1], sys.argv[2]

with open(infile, "r", encoding="utf-8", errors="replace") as f:
    lines = f.readlines()

def norm_line(line: str) -> str:
    # Don't mutate captured command output; Python's repr() choice is significant.
    if line.lstrip().startswith("output:"):
        return line

    # Boolean normalization: Python True/False -> Rust-style true/false.
    line = re.sub(r"\bTrue\b", "true", line)
    line = re.sub(r"\bFalse\b", "false", line)

    # Quote normalization for list-like values: ['a', 'b'] -> ["a", "b"].
    # Restrict to "key: [..]" patterns to avoid touching e.g. meta['k'] indexing.
    if ": [" in line:
        line = re.sub(
            r": \[(.*?)\]",
            lambda m: ": [" + m.group(1).replace("'", '"') + "]",
            line,
        )
    return line

with open(outfile, "w", encoding="utf-8") as f:
    for line in lines:
        f.write(norm_line(line))
PY
}

run_test() {
    local module="$1"
    local python_file="$PHASE_DIR/python/test_${module}.py"
    local tmp_python=$(mktemp)
    local tmp_rust=$(mktemp)

    echo -e "\n${CYAN}=== ${module^} Tests ===${NC}"

    if [ ! -f "$python_file" ]; then
        echo -e "${YELLOW}[SKIP] Python test not found: $python_file${NC}"
        rm -f "$tmp_python" "$tmp_rust"
        return
    fi

    # Run Python
    echo -n "Running Python... "
    if [ "${PARITY_ALLOW_CLICK_SRC}" = "1" ]; then
        "${PYTHON}" "$python_file" > "$tmp_python" 2>&1 || {
            echo -e "${RED}FAILED${NC}"
            cat "$tmp_python"
            rm -f "$tmp_python" "$tmp_rust"
            return 1
        }
    else
        env -u CLICK_SRC "${PYTHON}" "$python_file" > "$tmp_python" 2>&1 || {
            echo -e "${RED}FAILED${NC}"
            cat "$tmp_python"
            rm -f "$tmp_python" "$tmp_rust"
            return 1
        }
    fi
    echo "done"

    if [ "$PHASE" = "phase2" ] || [ "$PHASE" = "phase3" ]; then
        local tmp_norm=$(mktemp)
        normalize_phase2_output "$tmp_python" "$tmp_norm"
        mv "$tmp_norm" "$tmp_python"
    fi

    # Run Rust
    echo -n "Running Rust... "
    "$RUST_BIN" "$module" > "$tmp_rust" 2>&1 || {
        echo -e "${RED}FAILED${NC}"
        cat "$tmp_rust"
        rm -f "$tmp_python" "$tmp_rust"
        return 1
    }
    echo "done"

    # Compare outputs
    if diff -q "$tmp_python" "$tmp_rust" > /dev/null 2>&1; then
        echo -e "${GREEN}[PASS]${NC} Outputs match"
    else
        echo -e "${RED}[DIFF]${NC} Outputs differ:"
        diff --color=always -u "$tmp_python" "$tmp_rust" | head -80 || true
    fi

    rm -f "$tmp_python" "$tmp_rust"
}

# Get module list for phase
get_modules() {
    case "$PHASE" in
        phase1) echo "types errors" ;;
        phase2) echo "context parameter" ;;
        phase3) echo "parser command group" ;;
        phase4) echo "decorators formatting" ;;
        phase5) echo "termui" ;;
        phase6) echo "completion testing" ;;
        *) echo "" ;;
    esac
}

# Run tests
if [ "$MODULE" = "all" ]; then
    for mod in $(get_modules); do
        run_test "$mod"
    done
else
    run_test "$MODULE"
fi

echo -e "\n${CYAN}Done.${NC}"
