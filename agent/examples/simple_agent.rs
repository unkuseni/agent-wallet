//! Simple Agent Example
//!
//! This example demonstrates a basic deterministic agent that decides
//! to transfer SOL at regular intervals.

use agent_wallet_agent::prelude::*;
use agent_wallet_core::prelude::*;
use anyhow::Result;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting simple agent example...");

    // Create a deterministic agent with a periodic transfer strategy
    let agent = DeterministicAgent::new(DeterministicStrategy::PeriodicTransfer {
        interval_seconds: 3600, // Transfer every hour
        recipient: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"
            .parse()
            .expect("Invalid recipient address"),
        amount_sol: 0.1,
    });

    // Create agent context with current state
    let context = AgentContext {
        wallet_balance: 5.0,           // 5 SOL available
        market_data: None,             // No market data for this simple agent
        timestamp: Utc::now(),         // Current time
        transaction_count: 0,          // No transactions yet
        last_action_timestamp: None,   // No previous action
    };

    println!("Agent context created:");
    println!("  Wallet balance: {} SOL", context.wallet_balance);
    println!("  Timestamp: {}", context.timestamp);

    // Get agent decision
    match agent.decide(&context).await? {
        Some(decision) => {
            println!("\nAgent decided to take action:");
            match decision {
                AgentAction::TransferSol {
                    recipient,
                    amount_sol,
                    memo,
                } => {
                    println!("  Type: SOL Transfer");
                    println!("  Recipient: {}", recipient);
                    println!("  Amount: {} SOL", amount_sol);
                    if let Some(m) = memo {
                        println!("  Memo: {}", m);
                    }
                }
                _ => println!("  Other action: {:?}", decision),
            }

            // In a real implementation, you would now:
            // 1. Convert the AgentAction to Solana instructions
            // 2. Build a transaction using TransactionBuilder
            // 3. Sign and send the transaction using Wallet
            // 4. Monitor the transaction status

            println!("\nTo execute this action, you would:");
            println!("  1. Load a wallet with Wallet::load()");
            println!("  2. Build transaction with TransactionBuilder");
            println!("  3. Sign and send with wallet.sign_and_send()");
        }
        None => {
            println!("\nAgent decided no action is required at this time.");
            println!("This could mean:");
            println!("  - Not enough time has passed since last transfer");
            println!("  - Insufficient balance for the transfer");
            println!("  - Other strategy-specific conditions not met");
        }
    }

    // Demonstrate multiple decisions over time
    println!("\n--- Simulating multiple decision cycles ---");

    let mut simulated_context = context.clone();

    for i in 1..=3 {
        println!("\nDecision cycle {}:", i);

        // Simulate time passing (add 30 minutes each cycle)
        simulated_context.timestamp = simulated_context
            .timestamp
            .checked_add_signed(chrono::Duration::minutes(30))
            .unwrap();

        // Simulate balance change (add 0.5 SOL each cycle)
        simulated_context.wallet_balance += 0.5;

        match agent.decide(&simulated_context).await? {
            Some(action) => println!("  Action: {:?}", action),
            None => println!("  No action"),
        }
    }

    println!("\nExample completed successfully!");
    Ok(())
}
