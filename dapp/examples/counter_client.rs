//! Counter Client Example
//!
//! This example demonstrates how to interact with a simple counter program
//! on Solana using the AI Agent Wallet dApp library.

use agent_wallet_core::prelude::*;
use agent_wallet_dapp::prelude::*;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting counter client example...");

    // In a real scenario, you would:
    // 1. Deploy a counter program to devnet
    // 2. Create a counter account
    // 3. Use the program ID and account address

    // For this example, we'll use placeholder values
    let program_id: Pubkey = "Counter111111111111111111111111111111111111111"
        .parse()
        .expect("Invalid program ID");

    let counter_account: Pubkey = "CounterAccount111111111111111111111111111111111111"
        .parse()
        .expect("Invalid counter account");

    println!("Counter Program ID: {}", program_id);
    println!("Counter Account: {}", counter_account);

    // Create a mock RPC client (in reality, you'd use Wallet's RPC client)
    // For this example, we'll just demonstrate the API usage

    println!("\nExample API usage:");
    println!("------------------");

    // 1. Creating a counter client
    println!("\n1. Creating CounterClient:");
    println!("   let counter = CounterClient::new(program_id, rpc_client);");
    println!("   // The client is now ready to interact with the counter program");

    // 2. Incrementing the counter
    println!("\n2. Incrementing counter:");
    println!("   // Build increment instruction");
    println!("   let increment_ix = CounterInstruction::increment(counter_account);");
    println!("   ");
    println!("   // Create transaction");
    println!("   let mut transaction = Transaction::new_with_payer(");
    println!("       &[increment_ix],");
    println!("       Some(&wallet.pubkey())");
    println!("   );");
    println!("   ");
    println!("   // Sign and send");
    println!("   let signature = wallet.sign_and_send(transaction).await?;");
    println!("   println!(\"Counter incremented: {}\", signature);");

    // 3. Getting current count
    println!("\n3. Getting current count:");
    println!("   // Fetch account data");
    println!("   let count = counter.get_count(counter_account).await?;");
    println!("   println!(\"Current count: {}\", count);");

    // 4. Decrementing the counter (if supported)
    println!("\n4. Decrementing counter:");
    println!("   let decrement_ix = CounterInstruction::decrement(counter_account);");
    println!("   // Similar transaction building and signing process");

    // 5. Setting counter to specific value
    println!("\n5. Setting counter to specific value:");
    println!("   let set_ix = CounterInstruction::set(counter_account, 42);");
    println!("   // Transaction would set counter to value 42");

    // Real implementation example structure
    println!("\n\nReal implementation would look like:");
    println!("-------------------------------------");

    println!("\n#[tokio::main]");
    println!("async fn main() -> Result<()> {{");
    println!("    // Load wallet");
    println!("    let config = WalletConfig::new()");
    println!("        .with_rpc_url(\"https://api.devnet.solana.com\");");
    println!("    let wallet = Wallet::load(\"wallet.json\", config)?;");
    println!("    ");
    println!("    // Create counter client");
    println!("    let counter = CounterClient::new(program_id, wallet.rpc_client());");
    println!("    ");
    println!("    // Get current count");
    println!("    match counter.get_count(counter_account).await {{");
    println!("        Ok(count) => println!(\"Current count: {{}}\", count),");
    println!("        Err(e) => println!(\"Error fetching count: {{}}\", e),");
    println!("    }}");
    println!("    ");
    println!("    // Increment counter");
    println!("    let increment_ix = CounterInstruction::increment(counter_account);");
    println!("    let mut tx = Transaction::new_with_payer(");
    println!("        &[increment_ix],");
    println!("        Some(&wallet.pubkey())");
    println!("    );");
    println!("    ");
    println!("    // Get recent blockhash");
    println!("    let recent_blockhash = wallet.rpc_client()");
    println!("        .get_latest_blockhash()");
    println!("        .await?;");
    println!("    tx.sign(&[&wallet.keypair()], recent_blockhash);");
    println!("    ");
    println!("    // Send transaction");
    println!("    let signature = wallet.rpc_client()");
    println!("        .send_and_confirm_transaction(&tx)");
    println!("        .await?;");
    println!("    ");
    println!("    println!(\"Transaction sent: {{}}\", signature);");
    println!("    ");
    println!("    // Verify the increment worked");
    println!("    tokio::time::sleep(std::time::Duration::from_secs(2)).await;");
    println!("    let new_count = counter.get_count(counter_account).await?;");
    println!("    println!(\"New count: {{}}\", new_count);");
    println!("    ");
    println!("    Ok(())");
    println!("}}");

    // Error handling examples
    println!("\n\nError Handling:");
    println!("----------------");

    println!("\n// Different types of errors you might encounter:");
    println!("match counter.get_count(counter_account).await {{");
    println!("    Ok(count) => {{ /* success */ }}");
    println!("    Err(DappError::RpcError(e)) => {{");
    println!("        println!(\"RPC connection failed: {{}}\", e);");
    println!("        // Could retry with different RPC endpoint");
    println!("    }}");
    println!("    Err(DappError::AccountNotFound) => {{");
    println!("        println!(\"Counter account not found\");");
    println!("        // Might need to create the counter account first");
    println!("    }}");
    println!("    Err(DappError::InvalidAccountData) => {{");
    println!("        println!(\"Account data is not a valid counter\");");
    println!("        // Wrong account type or corrupted data");
    println!("    }}");
    println!("    Err(e) => {{");
    println!("        println!(\"Unexpected error: {{}}\", e);");
    println!("    }}");
    println!("}}");

    // Integration with agent framework
    println!("\n\nIntegration with Agent Framework:");
    println!("-----------------------------------");

    println!("\n// An agent could use the counter client like this:");
    println!("struct CounterAgent {{");
    println!("    counter_client: CounterClient,");
    println!("    counter_account: Pubkey,");
    println!("    target_count: u64,");
    println!("}}");
    println!("");
    println!("impl CounterAgent {{");
    println!("    async fn decide(&self, context: &AgentContext) -> Result<Option<AgentAction>> {{");
    println!("        let current_count = self.counter_client");
    println!("            .get_count(self.counter_account)");
    println!("            .await?;");
    println!("        ");
    println!("        if current_count < self.target_count {{");
    println!("            // Create increment action");
    println!("            let action = AgentAction::CustomProgramCall {{");
    println!("                program_id: self.counter_client.program_id(),");
    println!("                instruction: CounterInstruction::increment(self.counter_account),");
    println!("                accounts: vec![],");
    println!("            }};");
    println!("            Ok(Some(action))");
    println!("        }} else {{");
    println!("            Ok(None)");
    println!("        }}");
    println!("    }}");
    println!("}}");

    println!("\n\nExample completed!");
    println!("To run a real counter example:");
    println!("1. Deploy a counter program to devnet");
    println!("2. Create a counter account");
    println!("3. Update the program_id and counter_account values");
    println!("4. Use a real wallet with SOL for transaction fees");

    Ok(())
}
