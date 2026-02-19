#!/bin/bash

# start-devnet.sh - Start local Solana test validator for AI Agent Wallet development
#
# This script sets up a local Solana test environment with:
# - Local validator running on a specific port
# - Pre-funded test wallets for development
# - Optional deployment of test programs (counter program)
# - Environment configuration for agent wallet development

set -e

# Configuration
VALIDATOR_PORT=8899
RPC_PORT=8899
WEBSOCKET_PORT=8900
FAUCET_PORT=9900
LEDGER_DIR="./.ledger"
WALLETS_DIR="./wallets"
LOG_FILE="./validator.log"
TEST_PROGRAM_DIR="./test-programs"
COUNTER_PROGRAM_SO="counter.so"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Solana CLI is installed
check_solana_cli() {
    print_info "Checking Solana CLI installation..."
    if ! command -v solana &> /dev/null; then
        print_error "Solana CLI is not installed. Please install it first:"
        echo "  Visit: https://docs.solana.com/cli/install-solana-cli-tools"
        echo "  Or run: sh -c \"\$(curl -sSfL https://release.solana.com/stable/install)\""
        exit 1
    fi

    SOLANA_VERSION=$(solana --version)
    print_success "Solana CLI found: $SOLANA_VERSION"
}

# Check if test validator is already running
check_validator_running() {
    print_info "Checking if validator is already running on port $RPC_PORT..."
    if lsof -Pi :$RPC_PORT -sTCP:LISTEN -t >/dev/null ; then
        print_warning "Validator appears to be running on port $RPC_PORT"
        read -p "Do you want to stop it and start fresh? (y/N): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Stopping existing validator..."
            pkill -f "solana-test-validator" || true
            sleep 2
        else
            print_info "Using existing validator instance"
            return 1
        fi
    fi
    return 0
}

# Create directories if they don't exist
create_directories() {
    print_info "Creating necessary directories..."
    mkdir -p "$LEDGER_DIR"
    mkdir -p "$WALLETS_DIR"
    mkdir -p "$TEST_PROGRAM_DIR"
    print_success "Directories created"
}

# Generate test wallets
generate_test_wallets() {
    print_info "Generating test wallets..."

    # Agent wallet
    if [ ! -f "$WALLETS_DIR/agent-wallet.json" ]; then
        print_info "Creating agent wallet..."
        solana-keygen new --no-passphrase --force --outfile "$WALLETS_DIR/agent-wallet.json"
        print_success "Agent wallet created: $WALLETS_DIR/agent-wallet.json"
    else
        print_info "Agent wallet already exists: $WALLETS_DIR/agent-wallet.json"
    fi

    # Test receiver wallet
    if [ ! -f "$WALLETS_DIR/receiver-wallet.json" ]; then
        print_info "Creating receiver wallet..."
        solana-keygen new --no-passphrase --force --outfile "$WALLETS_DIR/receiver-wallet.json"
        print_success "Receiver wallet created: $WALLETS_DIR/receiver-wallet.json"
    else
        print_info "Receiver wallet already exists: $WALLETS_DIR/receiver-wallet.json"
    fi

    # Show wallet addresses
    AGENT_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/agent-wallet.json")
    RECEIVER_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/receiver-wallet.json")

    print_info "Agent wallet public key: $AGENT_PUBKEY"
    print_info "Receiver wallet public key: $RECEIVER_PUBKEY"
}

# Build test programs
build_test_programs() {
    print_info "Checking for test programs..."

    # Look for counter program source
    if [ -d "../programs/counter" ]; then
        print_info "Building counter program..."
        cd "../programs/counter"
        cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir="../../$TEST_PROGRAM_DIR"
        cd - > /dev/null
        print_success "Counter program built"
    elif [ -f "./counter/Cargo.toml" ]; then
        print_info "Building counter program from ./counter..."
        cd "./counter"
        cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir="../$TEST_PROGRAM_DIR"
        cd - > /dev/null
        print_success "Counter program built"
    else
        print_warning "Counter program source not found. Using pre-built if available."
    fi

    # Check if counter.so exists
    if [ -f "$TEST_PROGRAM_DIR/$COUNTER_PROGRAM_SO" ]; then
        print_success "Counter program found: $TEST_PROGRAM_DIR/$COUNTER_PROGRAM_SO"
    else
        print_warning "Counter program not found. You may need to build it manually."
    fi
}

# Start the test validator
start_validator() {
    print_info "Starting Solana test validator..."

    # Set Solana config to local
    solana config set --url http://127.0.0.1:$RPC_PORT

    # Build validator command
    VALIDATOR_CMD="solana-test-validator \
        --ledger $LEDGER_DIR \
        --rpc-port $RPC_PORT \
        --faucet-port $FAUCET_PORT \
        --quiet \
        --reset \
        --limit-ledger-size"

    # Add known programs if they exist
    if [ -f "$TEST_PROGRAM_DIR/$COUNTER_PROGRAM_SO" ]; then
        COUNTER_PROGRAM_ID=$(solana address -k "$TEST_PROGRAM_DIR/counter-keypair.json" 2>/dev/null || echo "")
        if [ -n "$COUNTER_PROGRAM_ID" ]; then
            VALIDATOR_CMD="$VALIDATOR_CMD --bpf-program $COUNTER_PROGRAM_ID $TEST_PROGRAM_DIR/$COUNTER_PROGRAM_SO"
            print_info "Counter program will be deployed with ID: $COUNTER_PROGRAM_ID"
        else
            print_warning "Counter program keypair not found, program won't be pre-deployed"
        fi
    fi

    # Start validator in background
    print_info "Starting validator with command:"
    echo "  $VALIDATOR_CMD"

    # Run validator in background and log output
    $VALIDATOR_CMD > "$LOG_FILE" 2>&1 &
    VALIDATOR_PID=$!

    # Wait for validator to start
    print_info "Waiting for validator to start (max 30 seconds)..."

    for i in {1..30}; do
        if curl -s -X POST -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}' \
            http://127.0.0.1:$RPC_PORT > /dev/null 2>&1; then
            print_success "Validator started successfully (PID: $VALIDATOR_PID)"
            echo $VALIDATOR_PID > "$LEDGER_DIR/validator.pid"
            return 0
        fi
        sleep 1
        echo -n "."
    done

    print_error "Validator failed to start within 30 seconds"
    print_error "Check logs: $LOG_FILE"
    return 1
}

# Fund test wallets
fund_test_wallets() {
    print_info "Funding test wallets..."

    # Wait a bit for validator to be ready
    sleep 2

    # Fund agent wallet
    AGENT_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/agent-wallet.json")
    print_info "Funding agent wallet: $AGENT_PUBKEY"
    solana airdrop 100 $AGENT_PUBKEY || {
        print_warning "Failed to airdrop to agent wallet, retrying..."
        sleep 2
        solana airdrop 100 $AGENT_PUBKEY
    }

    # Fund receiver wallet
    RECEIVER_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/receiver-wallet.json")
    print_info "Funding receiver wallet: $RECEIVER_PUBKEY"
    solana airdrop 50 $RECEIVER_PUBKEY || {
        print_warning "Failed to airdrop to receiver wallet, retrying..."
        sleep 2
        solana airdrop 50 $RECEIVER_PUBKEY
    }

    print_success "Test wallets funded"
}

# Deploy test programs
deploy_test_programs() {
    print_info "Deploying test programs..."

    # Deploy counter program if it exists
    if [ -f "$TEST_PROGRAM_DIR/$COUNTER_PROGRAM_SO" ]; then
        print_info "Deploying counter program..."

        # Check if program is already deployed by validator
        sleep 2

        # If not pre-deployed by validator, deploy it manually
        COUNTER_PROGRAM_ID=$(solana program deploy \
            --program-id "$TEST_PROGRAM_DIR/counter-keypair.json" \
            "$TEST_PROGRAM_DIR/$COUNTER_PROGRAM_SO" \
            --url http://127.0.0.1:$RPC_PORT 2>/dev/null | grep "Program Id:" | awk '{print $3}' || echo "")

        if [ -n "$COUNTER_PROGRAM_ID" ]; then
            print_success "Counter program deployed: $COUNTER_PROGRAM_ID"

            # Create counter account
            print_info "Creating counter account..."
            AGENT_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/agent-wallet.json")
            COUNTER_ACCOUNT=$(solana create-account --from "$WALLETS_DIR/agent-wallet.json" \
                --owner "$COUNTER_PROGRAM_ID" \
                --lamports 1000000000 \
                --url http://127.0.0.1:$RPC_PORT 2>/dev/null | grep "Created account" | awk '{print $3}' || echo "")

            if [ -n "$COUNTER_ACCOUNT" ]; then
                print_success "Counter account created: $COUNTER_ACCOUNT"
                echo "COUNTER_PROGRAM_ID=$COUNTER_PROGRAM_ID" > "$WALLETS_DIR/counter.env"
                echo "COUNTER_ACCOUNT=$COUNTER_ACCOUNT" >> "$WALLETS_DIR/counter.env"
            fi
        else
            print_warning "Counter program deployment may have failed or already deployed"
        fi
    fi
}

# Set up environment
setup_environment() {
    print_info "Setting up environment..."

    # Create environment file
    AGENT_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/agent-wallet.json")
    RECEIVER_PUBKEY=$(solana-keygen pubkey "$WALLETS_DIR/receiver-wallet.json")

    cat > ".env.devnet" << EOF
# AI Agent Wallet Development Environment
# Generated by start-devnet.sh

# Solana Network
SOLANA_RPC_URL=http://127.0.0.1:$RPC_PORT
SOLANA_WS_URL=ws://127.0.0.1:$WEBSOCKET_PORT
SOLANA_NETWORK=localnet

# Test Wallets
AGENT_WALLET_PATH=$WALLETS_DIR/agent-wallet.json
AGENT_WALLET_PUBKEY=$AGENT_PUBKEY
RECEIVER_WALLET_PATH=$WALLETS_DIR/receiver-wallet.json
RECEIVER_WALLET_PUBKEY=$RECEIVER_PUBKEY

# Counter Program (if deployed)
$(cat "$WALLETS_DIR/counter.env" 2>/dev/null || echo "# Counter program not deployed")

# Configuration
WALLET_ENCRYPTION_PASSPHRASE="devnet-test-passphrase"
AGENT_PERMISSION_LEVEL="full"
DAILY_SPEND_LIMIT_SOL=1000.0
EOF

    print_success "Environment file created: .env.devnet"

    # Show configuration
    print_info "Current Solana configuration:"
    solana config get
}

# Show usage information
show_usage() {
    print_info "AI Agent Wallet Devnet Starter"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -h, --help      Show this help message"
    echo "  -c, --clean     Clean ledger and wallet directories before starting"
    echo "  -b, --build     Build test programs before starting"
    echo "  -d, --deploy    Deploy test programs after starting validator"
    echo "  -q, --quiet     Quiet mode (minimal output)"
    echo ""
    echo "Environment:"
    echo "  Set VALIDATOR_PORT to change RPC port (default: 8899)"
    echo "  Set LEDGER_DIR to change ledger directory (default: ./.ledger)"
    echo "  Set WALLETS_DIR to change wallets directory (default: ./wallets)"
}

# Clean up directories
clean_directories() {
    print_info "Cleaning directories..."
    if [ -d "$LEDGER_DIR" ]; then
        rm -rf "$LEDGER_DIR"
        print_success "Removed ledger directory: $LEDGER_DIR"
    fi
    if [ -d "$WALLETS_DIR" ]; then
        rm -rf "$WALLETS_DIR"
        print_success "Removed wallets directory: $WALLETS_DIR"
    fi
    if [ -f "$LOG_FILE" ]; then
        rm -f "$LOG_FILE"
        print_success "Removed log file: $LOG_FILE"
    fi
}

# Main function
main() {
    # Parse command line arguments
    CLEAN=false
    BUILD=false
    DEPLOY=false
    QUIET=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -c|--clean)
                CLEAN=true
                shift
                ;;
            -b|--build)
                BUILD=true
                shift
                ;;
            -d|--deploy)
                DEPLOY=true
                shift
                ;;
            -q|--quiet)
                QUIET=true
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    # Check prerequisites
    check_solana_cli

    # Clean if requested
    if [ "$CLEAN" = true ]; then
        clean_directories
    fi

    # Check if validator is already running
    if check_validator_running; then
        # Create directories
        create_directories

        # Generate wallets
        generate_test_wallets

        # Build programs if requested
        if [ "$BUILD" = true ]; then
            build_test_programs
        fi

        # Start validator
        if start_validator; then
            # Fund wallets
            fund_test_wallets

            # Deploy programs if requested
            if [ "$DEPLOY" = true ]; then
                deploy_test_programs
            fi

            # Set up environment
            setup_environment

            # Show final information
            echo ""
            print_success "========================================"
            print_success "AI Agent Wallet Devnet is READY!"
            print_success "========================================"
            echo ""
            echo "RPC URL:      http://127.0.0.1:$RPC_PORT"
            echo "WebSocket:    ws://127.0.0.1:$WEBSOCKET_PORT"
            echo "Faucet:       http://127.0.0.1:$FAUCET_PORT"
            echo ""
            echo "Agent Wallet: $WALLETS_DIR/agent-wallet.json"
            echo "Receiver:     $WALLETS_DIR/receiver-wallet.json"
            echo ""
            echo "Environment:  .env.devnet"
            echo "Log file:     $LOG_FILE"
            echo "Validator PID: $(cat "$LEDGER_DIR/validator.pid" 2>/dev/null || echo "Unknown")"
            echo ""
            echo "To use this devnet with agent-wallet:"
            echo "  export SOLANA_RPC_URL=http://127.0.0.1:$RPC_PORT"
            echo "  source .env.devnet"
            echo ""
            print_info "Press Ctrl+C to stop the validator"
            echo ""

            # Keep script running if not in quiet mode
            if [ "$QUIET" = false ]; then
                print_info "Validator output (tail -f $LOG_FILE):"
                echo "----------------------------------------"
                tail -f "$LOG_FILE"
            fi
        else
            print_error "Failed to start validator"
            exit 1
        fi
    else
        print_info "Using existing validator instance"
        setup_environment

        echo ""
        print_success "Existing validator detected and configured"
        echo "RPC URL: http://127.0.0.1:$RPC_PORT"
        echo ""
        print_info "To stop the validator: pkill -f 'solana-test-validator'"
    fi
}

# Handle script interruption
cleanup() {
    print_info "Shutting down..."
    if [ -f "$LEDGER_DIR/validator.pid" ]; then
        VALIDATOR_PID=$(cat "$LEDGER_DIR/validator.pid")
        print_info "Stopping validator (PID: $VALIDATOR_PID)"
        kill $VALIDATOR_PID 2>/dev/null || true
        rm -f "$LEDGER_DIR/validator.pid"
    fi
    print_success "Cleanup complete"
    exit 0
}

# Set up trap for script interruption
trap cleanup SIGINT SIGTERM

# Run main function
main "$@"
