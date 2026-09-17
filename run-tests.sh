#!/bin/bash

# ez-vend Test Runner
# Runs automated tests for the project.

set -e

echo "🧪 ez-vend Test Suite"
echo "===================="
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

RUN_CHROME=false
RUN_SAFARI=false
SHOW_HELP=false

for arg in "$@"; do
    case "$arg" in
        --chrome)
            RUN_CHROME=true
            ;;
        --safari)
            RUN_SAFARI=true
            ;;
        --help)
            SHOW_HELP=true
            ;;
        *)
            echo "Unknown option: $arg"
            SHOW_HELP=true
            ;;
    esac
done

if [ "$SHOW_HELP" = true ]; then
    echo "Usage: ./run-tests.sh [--chrome] [--safari] [--help]"
    echo ""
    echo "  (no flags)      Run unit tests only"
    echo "  --chrome        Include Chrome browser integration tests"
    echo "  --safari        Include Safari browser integration tests"
    echo "  --help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./run-tests.sh"
    echo "  ./run-tests.sh --chrome"
    echo "  ./run-tests.sh --safari"
    echo "  ./run-tests.sh --chrome --safari"
    exit 0
fi

WASM_PACK_AVAILABLE=false
if command -v wasm-pack >/dev/null 2>&1; then
    WASM_PACK_AVAILABLE=true
elif [ "$RUN_CHROME" = true ] || [ "$RUN_SAFARI" = true ]; then
    echo "⚠️  wasm-pack not found. Install it with:"
    echo "   cargo install wasm-pack"
    echo ""
    echo "Skipping requested browser tests..."
    RUN_CHROME=false
    RUN_SAFARI=false
fi

printf "%b\n" "${BLUE}📦 Running Unit Tests${NC}"
echo "-------------------"
cargo test --workspace --lib
echo ""

if [ "$RUN_CHROME" = true ] && [ "$WASM_PACK_AVAILABLE" = true ]; then
    printf "%b\n" "${BLUE}🌐 Running Chrome Browser Tests${NC}"
    echo "--------------------------------"
    if [ -x "./scripts/ensure-chromedriver.sh" ]; then
        CHROMEDRIVER_PATH="$(./scripts/ensure-chromedriver.sh)"
        export CHROMEDRIVER="$CHROMEDRIVER_PATH"
        printf "%b\n" "${YELLOW}Using ChromeDriver: $CHROMEDRIVER_PATH${NC}"
    fi
    wasm-pack test --headless --chrome crates/storage
    wasm-pack test --headless --chrome crates/ez-vend-ui
    echo ""
fi

if [ "$RUN_SAFARI" = true ] && [ "$WASM_PACK_AVAILABLE" = true ]; then
    printf "%b\n" "${BLUE}🧭 Running Safari Browser Tests${NC}"
    echo "--------------------------------"
    if [ -x "./scripts/ensure-safaridriver.sh" ]; then
        ./scripts/ensure-safaridriver.sh >/dev/null
        printf "%b\n" "${YELLOW}Safari WebDriver ready${NC}"
    fi
    wasm-pack test --headless --safari crates/storage
    wasm-pack test --headless --safari crates/ez-vend-ui
    echo ""
fi

printf "%b\n" "${GREEN}✅ All tests passed!${NC}"
echo ""
echo "📊 Test Summary:"
UNIT_TEST_COUNT="$(cargo test --workspace --lib -- --list 2>/dev/null | grep -E ': test$' | wc -l | xargs)"
echo "  Unit tests: $UNIT_TEST_COUNT"

if [ "$RUN_CHROME" = true ]; then
    echo "  Chrome integration tests: 13"
fi

if [ "$RUN_SAFARI" = true ]; then
    echo "  Safari integration tests: 17"
fi

if [ "$RUN_CHROME" = false ] && [ "$RUN_SAFARI" = false ]; then
    echo ""
    echo "💡 Tip: run browser suites with --chrome and/or --safari"
fi

echo ""
echo "For more testing options, see TESTING.md"
