#!/bin/bash

# deploy-test-program.sh - Deploy test program to devnet for AI Agent Wallet development
#
# This script builds and deploys a simple counter program to devnet
# for testing AI agent wallet interactions.

set -e

# Configuration
PROGRAM_NAME="counter"
PROGRAM_SRC_DIR="./programs/$PROGRAM_NAME"
PROGRAM_BUILD_DIR="./target/deploy"
DEVNET_URL="https://api.devnet.solana.com"
LOCALNET_URL="http://127.0.0.1:8899"
ENV_FILE=".env.program"
KEYPAIR_FILE="$PROGRAM_BUILD_DIR/${PROGRAM_NAME}-keypair.json"
PROGRAM_SO_FILE="$PROGRAM_BUILD_DIR/$PROGRAM_NAME.so"
WALLETS_DIR="./wallets"
DEFAULT_WALLET="$WALLETS_DIR/agent-wallet.json"

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

# Check prerequisites
check_prerequisites() {
    print_info "Checking prerequisites..."

    # Check Solana CLI
    if ! command -v solana &> /dev/null; then
        print_error "Solana CLI is not installed."
        echo "Install with: sh -c \"\$(curl -sSfL https://release.solana.com/stable/install)\""
        exit 1
    fi

    # Check Rust and Cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo is not installed."
        echo "Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    # Check solana-program-build
    if ! command -v cargo-build-bpf &> /dev/null; then
        print_info "Installing solana-program-build..."
        cargo install solana-program-build
    fi

    SOLANA_VERSION=$(solana --version)
    print_success "Solana CLI: $SOLANA_VERSION"
}

# Detect network and URL
detect_network() {
    local current_url=$(solana config get | grep "RPC URL" | awk '{print $3}')

    case $current_url in
        *devnet*)
            NETWORK="devnet"
            URL=$DEVNET_URL
            ;;
        *localhost*|*127.0.0.1*)
            NETWORK="localnet"
            URL=$LOCALNET_URL
            ;;
        *)
            print_warning "Unknown RPC URL: $current_url"
            print_info "Using devnet as default"
            NETWORK="devnet"
            URL=$DEVNET_URL
            ;;
    esac

    print_info "Detected network: $NETWORK"
    print_info "RPC URL: $URL"
}

# Build the program
build_program() {
    print_info "Building $PROGRAM_NAME program..."

    # Check if source directory exists
    if [ ! -d "$PROGRAM_SRC_DIR" ]; then
        print_error "Program source directory not found: $PROGRAM_SRC_DIR"
        print_info "Looking for alternative locations..."

        # Try to find the program source
        if [ -d "./counter" ]; then
            PROGRAM_SRC_DIR="./counter"
            print_info "Found program at: $PROGRAM_SRC_DIR"
        elif [ -d "../programs/counter" ]; then
            PROGRAM_SRC_DIR="../programs/counter"
            print_info "Found program at: $PROGRAM_SRC_DIR"
        else
            print_error "Could not find program source. Please create a counter program first."
            exit 1
        fi
    fi

    # Build the program
    cd "$PROGRAM_SRC_DIR"
    print_info "Building in directory: $(pwd)"

    cargo build-bpf --manifest-path=./Cargo.toml --bpf-out-dir="../$PROGRAM_BUILD_DIR"

    cd - > /dev/null

    # Check if build was successful
    if [ ! -f "$PROGRAM_SO_FILE" ]; then
        print_error "Build failed: $PROGRAM_SO_FILE not found"
        exit 1
    fi

    print_success "Program built successfully: $PROGRAM_SO_FILE"
    print_info "Program size: $(ls -lh "$PROGRAM_SO_FILE" | awk '{print $5}')"
}

# Check if wallet exists and has funds
check_wallet() {
    local wallet_path=$1

    if [ ! -f "$wallet_path" ]; then
        print_error "Wallet not found: $wallet_path"
        print_info "Available wallets in $WALLETS_DIR:"
        ls -la "$WALLETS_DIR/" 2>/dev/null || echo "No wallets directory found"

        # Try to use default solana keypair
        DEFAULT_KEYPAIR=~/.config/solana/id.json
        if [ -f "$DEFAULT_KEYPAIR" ]; then
            print_info "Using default Solana keypair: $DEFAULT_KEYPAIR"
            echo "$DEFAULT_KEYPAIR"
        else
            print_error "No wallet available. Please create one first."
            echo "  solana-keygen new -o $wallet_path"
            echo "  solana airdrop 1 <pubkey> --url $URL"
            exit 1
        fi
    else
        echo "$wallet_path"
    fi
}

# Deploy the program
deploy_program() {
    local wallet_path=$1

    print_info "Deploying $PROGRAM_NAME program to $NETWORK..."

    # Get program ID from keypair
    if [ ! -f "$KEYPAIR_FILE" ]; then
        print_error "Program keypair not found: $KEYPAIR_FILE"
        print_info "Generating new program keypair..."
        solana-keygen new --no-passphrase --force --outfile "$KEYPAIR_FILE"
    fi

    PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR_FILE")
    print_info "Program ID: $PROGRAM_ID"

    # Check if program is already deployed
    print_info "Checking if program is already deployed..."
    if solana program show "$PROGRAM_ID" --url "$URL" &> /dev/null; then
        print_warning "Program already deployed at $PROGRAM_ID"
        read -p "Redeploy? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Using existing deployment"
            echo "$PROGRAM_ID"
            return 0
        fi
    fi

    # Deploy the program
    print_info "Deploying program (this may take a minute)..."

    # Check wallet balance
    WALLET_PUBKEY=$(solana-keygen pubkey "$wallet_path")
    BALANCE=$(solana balance "$WALLET_PUBKEY" --url "$URL")
    print_info "Deployer wallet: $WALLET_PUBKEY"
    print_info "Balance: $BALANCE"

    # Deploy with retry logic
    for attempt in {1..3}; do
        print_info "Deployment attempt $attempt/3..."

        DEPLOY_OUTPUT=$(solana program deploy \
            --program-id "$KEYPAIR_FILE" \
            "$PROGRAM_SO_FILE" \
            --keypair "$wallet_path" \
            --url "$URL" 2>&1)

        if echo "$DEPLOY_OUTPUT" | grep -q "Program Id:"; then
            DEPLOYED_ID=$(echo "$DEPLOY_OUTPUT" | grep "Program Id:" | awk '{print $3}')
            print_success "Program deployed successfully: $DEPLOYED_ID"
            echo "$DEPLOYED_ID"
            return 0
        else
            print_warning "Deployment attempt $attempt failed"
            if [ $attempt -lt 3 ]; then
                print_info "Retrying in 5 seconds..."
                sleep 5

                # Airdrop more funds if needed
                if echo "$DEPLOY_OUTPUT" | grep -q "Insufficient funds"; then
                    print_info "Requesting airdrop..."
                    solana airdrop 1 "$WALLET_PUBKEY" --url "$URL" || true
                fi
            fi
        fi
    done

    print_error "Failed to deploy program after 3 attempts"
    echo "Last error:"
    echo "$DEPLOY_OUTPUT"
    exit 1
}

# Create counter account
create_counter_account() {
    local program_id=$1
    local wallet_path=$2

    print_info "Creating counter account..."

    # Generate account keypair
    COUNTER_ACCOUNT_KEYPAIR="$PROGRAM_BUILD_DIR/counter-account-keypair.json"
    solana-keygen new --no-passphrase --force --outfile "$COUNTER_ACCOUNT_KEYPAIR"
    COUNTER_ACCOUNT=$(solana-keygen pubkey "$COUNTER_ACCOUNT_KEYPAIR")

    print_info "Counter account address: $COUNTER_ACCOUNT"

    # Create and fund the account
    print_info "Creating account with 0.1 SOL..."

    solana create-account \
        "$COUNTER_ACCOUNT" \
        100000000 \
        --owner "$program_id" \
        --keypair "$wallet_path" \
        --url "$URL"

    if [ $? -eq 0 ]; then
        print_success "Counter account created: $COUNTER_ACCOUNT"
        echo "$COUNTER_ACCOUNT"
    else
        print_error "Failed to create counter account"
        exit 1
    fi
}

# Save environment configuration
save_environment() {
    local program_id=$1
    local counter_account=$2

    print_info "Saving environment configuration..."

    cat > "$ENV_FILE" << EOF
# AI Agent Wallet Test Program Configuration
# Generated by deploy-test-program.sh
# $(date)

# Program Configuration
COUNTER_PROGRAM_ID=$program_id
COUNTER_ACCOUNT=$counter_account
COUNTER_PROGRAM_SO=$PROGRAM_SO_FILE
COUNTER_KEYPAIR=$KEYPAIR_FILE
COUNTER_ACCOUNT_KEYPAIR=$PROGRAM_BUILD_DIR/counter-account-keypair.json

# Network Configuration
SOLANA_NETWORK=$NETWORK
SOLANA_RPC_URL=$URL

# Example Usage
# To interact with the counter program:
# 1. Use COUNTER_PROGRAM_ID as the program ID
# 2. Use COUNTER_ACCOUNT as the counter state account
# 3. Instructions:
#    - Increment: instruction data [0]
#    - Decrement: instruction data [1]
#    - Set value: instruction data [2] + little-endian u64

# Integration with agent-wallet
# In your agent code:
# use solana_sdk::instruction::{AccountMeta, Instruction};
#
# let increment_ix = Instruction::new_with_bytes(
#     COUNTER_PROGRAM_ID,
#     &[0], // Increment instruction
#     vec![
#         AccountMeta::new(COUNTER_ACCOUNT, false),
#         AccountMeta::new_readonly(signer_pubkey, true),
#     ],
# );
EOF

    print_success "Environment saved: $ENV_FILE"
}

# Show usage information
show_usage() {
    print_info "Test Program Deployer"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -h, --help          Show this help message"
    echo "  -w, --wallet PATH   Use specific wallet file (default: $DEFAULT_WALLET)"
    echo "  -n, --network NET   Network: devnet or localnet (default: auto-detect)"
    echo "  -b, --build-only    Build program without deploying"
    echo "  -d, --deploy-only   Deploy without building (use existing build)"
    echo "  -c, --clean         Clean build directory before building"
    echo ""
    echo "Examples:"
    echo "  $0                     # Auto-detect network, build and deploy"
    echo "  $0 --network devnet    # Deploy to devnet"
    echo "  $0 --network localnet  # Deploy to localnet"
    echo "  $0 --wallet ./mywallet.json"
    echo ""
    echo "Environment variables:"
    echo "  SOLANA_RPC_URL      Override RPC URL"
    echo "  WALLETS_DIR         Override wallets directory"
}

# Clean build directory
clean_build() {
    print_info "Cleaning build directory..."
    if [ -d "$PROGRAM_BUILD_DIR" ]; then
        rm -rf "$PROGRAM_BUILD_DIR"
        print_success "Cleaned: $PROGRAM_BUILD_DIR"
    fi
}

# Main function
main() {
    # Parse command line arguments
    BUILD=true
    DEPLOY=true
    WALLET_PATH=$DEFAULT_WALLET
    CLEAN=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -w|--wallet)
                WALLET_PATH="$2"
                shift 2
                ;;
            -n|--network)
                case $2 in
                    devnet)
                        URL=$DEVNET_URL
                        NETWORK="devnet"
                        ;;
                    localnet)
                        URL=$LOCALNET_URL
                        NETWORK="localnet"
                        ;;
                    *)
                        print_error "Invalid network: $2"
                        show_usage
                        exit 1
                        ;;
                esac
                shift 2
                ;;
            -b|--build-only)
                DEPLOY=false
                shift
                ;;
            -d|--deploy-only)
                BUILD=false
                shift
                ;;
            -c|--clean)
                CLEAN=true
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done

    echo ""
    print_info "=== AI Agent Wallet Test Program Deployer ==="
    echo ""

    # Check prerequisites
    check_prerequisites

    # Detect network if not specified
    if [ -z "$NETWORK" ]; then
        detect_network
    fi

    # Set Solana config to detected URL
    solana config set --url "$URL"

    # Clean if requested
    if [ "$CLEAN" = true ]; then
        clean_build
    fi

    # Build program
    if [ "$BUILD" = true ]; then
        build_program
    else
        print_info "Skipping build (--deploy-only)"
        if [ ! -f "$PROGRAM_SO_FILE" ]; then
            print_error "Program not found: $PROGRAM_SO_FILE"
            print_info "Run without --deploy-only to build first"
            exit 1
        fi
    fi

    # Deploy program
    if [ "$DEPLOY" = true ]; then
        # Check wallet
        WALLET=$(check_wallet "$WALLET_PATH")

        # Deploy program
        PROGRAM_ID=$(deploy_program "$WALLET")

        # Create counter account
        COUNTER_ACCOUNT=$(create_counter_account "$PROGRAM_ID" "$WALLET")

        # Save environment
        save_environment "$PROGRAM_ID" "$COUNTER_ACCOUNT"

        # Show summary
        echo ""
        print_success "========================================"
        print_success "DEPLOYMENT COMPLETE!"
        print_success "========================================"
        echo ""
        echo "Program ID:      $PROGRAM_ID"
        echo "Counter Account: $COUNTER_ACCOUNT"
        echo "Network:         $NETWORK"
        echo "RPC URL:         $URL"
        echo "Environment:     $ENV_FILE"
        echo ""
        echo "To test the deployment:"
        echo "  source $ENV_FILE"
        echo "  solana program show \$COUNTER_PROGRAM_ID --url \$SOLANA_RPC_URL"
        echo "  solana account \$COUNTER_ACCOUNT --url \$SOLANA_RPC_URL"
        echo ""
        print_info "Add these to your agent-wallet configuration!"
    else
        print_info "Build complete. To deploy, run:"
        echo "  $0 --deploy-only --wallet $WALLET_PATH"
        echo ""
        print_info "Program binary: $PROGRAM_SO_FILE"
    fi
}

# Run main function
main "$@"
