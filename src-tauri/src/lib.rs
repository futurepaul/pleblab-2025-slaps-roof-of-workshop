// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tauri::Emitter;

// BDK wallet imports
use bdk_esplora::{esplora_client, EsploraAsyncExt};
use bdk_wallet::{
    bitcoin::{Amount, Network},
    rusqlite::Connection,
    KeychainKind, SignOptions, Wallet,
};

// Constants for BDK wallet
const DB_PATH: &str = "bdk-wallet.sqlite";
const NETWORK: Network = Network::Signet;
const EXTERNAL_DESC: &str = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/1'/0'/0/*)";
const INTERNAL_DESC: &str = "wpkh(tprv8ZgxMBicQKsPdy6LMhUtFHAgpocR8GC6QmwMSFpZs7h6Eziw3SpThFfczTDh5rW2krkqffa11UpX3XkeTTB2FvzZKWXqPY54Y6Rq4AQ5R8L/84'/1'/0'/1/*)";
const ESPLORA_URL: &str = "http://signet.bitcoindevkit.net";
const STOP_GAP: usize = 5;
const PARALLEL_REQUESTS: usize = 5;

// Define channel message type
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AppMessage {
    Ping,
    UpdateData(String),
    Shutdown,
    // Wallet operations
    GetWalletAddress,
    SyncWallet,
    GetWalletBalance,
    SendTransaction(u64), // Amount in sats
}

// Define app state to hold channel senders
#[derive(Debug)]
pub struct AppState {
    tx: mpsc::Sender<AppMessage>,
    wallet_path: String,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Command to send messages to the background task
#[tauri::command]
async fn send_to_background(
    state: tauri::State<'_, AppState>,
    message: AppMessage,
) -> Result<(), String> {
    state.tx.send(message)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create channel for communication with background task
    let (tx, mut rx) = mpsc::channel::<AppMessage>(100);
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { 
            tx: tx.clone(),
            wallet_path: DB_PATH.to_string(),
        })
        .setup(|app| {
            // Get app handle for sending events back to frontend
            let app_handle = app.handle().clone();
            
            // Spawn background task
            tauri::async_runtime::spawn(async move {
                println!("Background task started");
                
                // Heartbeat counter
                let mut heartbeat_count = 0;
                
                loop {
                    tokio::select! {
                        Some(message) = rx.recv() => {
                            match message {
                                AppMessage::Ping => {
                                    println!("Ping received!");
                                    // Send a pong response using window-specific API
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let _ = window.emit("background-event", "pong");
                                    }
                                },
                                AppMessage::UpdateData(data) => {
                                    println!("Data update: {}", data);
                                    // Send data update event
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let _ = window.emit("data-updated", data);
                                    }
                                },
                                AppMessage::Shutdown => {
                                    println!("Shutting down background task");
                                    break;
                                }
                                AppMessage::GetWalletAddress => {
                                    println!("Getting wallet address");
                                    
                                    // Open the wallet database
                                    match Connection::open(DB_PATH) {
                                        Ok(mut conn) => {
                                            // Try to load the wallet or create a new one
                                            let wallet_result = Wallet::load()
                                                .descriptor(KeychainKind::External, Some(EXTERNAL_DESC))
                                                .descriptor(KeychainKind::Internal, Some(INTERNAL_DESC))
                                                .extract_keys()
                                                .check_network(NETWORK)
                                                .load_wallet(&mut conn);
                                                
                                            match wallet_result {
                                                Ok(wallet_opt) => {
                                                    let mut wallet = match wallet_opt {
                                                        Some(wallet) => wallet,
                                                        None => match Wallet::create(EXTERNAL_DESC, INTERNAL_DESC)
                                                            .network(NETWORK)
                                                            .create_wallet(&mut conn) {
                                                                Ok(w) => w,
                                                                Err(e) => {
                                                                    let error_msg = format!("Failed to create wallet: {}", e);
                                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                                        let _ = window.emit("wallet-error", error_msg);
                                                                    }
                                                                    continue;
                                                                }
                                                            }
                                                    };
                                                    
                                                    // Get the next unused address
                                                    let address = wallet.next_unused_address(KeychainKind::External);
                                                    
                                                    // Persist changes to the wallet
                                                    if let Err(e) = wallet.persist(&mut conn) {
                                                        let error_msg = format!("Failed to persist wallet: {}", e);
                                                        if let Some(window) = app_handle.get_webview_window("main") {
                                                            let _ = window.emit("wallet-error", error_msg);
                                                        }
                                                        continue;
                                                    }
                                                    
                                                    // Send the address to the frontend
                                                    let address_info = format!("{}|{}", address.index, address);
                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                        let _ = window.emit("wallet-address", address_info);
                                                    }
                                                }
                                                Err(e) => {
                                                    let error_msg = format!("Failed to load wallet: {}", e);
                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                        let _ = window.emit("wallet-error", error_msg);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to open wallet database: {}", e);
                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                let _ = window.emit("wallet-error", error_msg);
                                            }
                                        }
                                    }
                                },
                                AppMessage::SyncWallet => {
                                    println!("Syncing wallet");
                                    
                                    // Open the wallet database
                                    match Connection::open(DB_PATH) {
                                        Ok(mut conn) => {
                                            // Try to load the wallet
                                            let wallet_result = Wallet::load()
                                                .descriptor(KeychainKind::External, Some(EXTERNAL_DESC))
                                                .descriptor(KeychainKind::Internal, Some(INTERNAL_DESC))
                                                .extract_keys()
                                                .check_network(NETWORK)
                                                .load_wallet(&mut conn);
                                                
                                            match wallet_result {
                                                Ok(wallet_opt) => {
                                                    let mut wallet = match wallet_opt {
                                                        Some(wallet) => wallet,
                                                        None => {
                                                            let error_msg = "Wallet not found. Create a wallet first.";
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("wallet-error", error_msg);
                                                            }
                                                            continue;
                                                        }
                                                    };
                                                    
                                                    // Create esplora client
                                                    let client_result = esplora_client::Builder::new(ESPLORA_URL).build_async();
                                                    match client_result {
                                                        Ok(client) => {
                                                            // Start sync process
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("sync-started", "Sync started");
                                                            }
                                                            
                                                            // Clone app_handle for closure
                                                            let closure_app_handle = app_handle.clone();
                                                            
                                                            let request = wallet.start_full_scan().inspect({
                                                                move |keychain, spk_i, _| {
                                                                    let sync_info = format!("Scanning keychain {:?} at index {}", keychain, spk_i);
                                                                    if let Some(window) = closure_app_handle.get_webview_window("main") {
                                                                        let _ = window.emit("sync-progress", sync_info);
                                                                    }
                                                                }
                                                            });
                                                            
                                                            match client.full_scan(request, STOP_GAP, PARALLEL_REQUESTS).await {
                                                                Ok(update) => {
                                                                    match wallet.apply_update(update) {
                                                                        Ok(_) => {
                                                                            if let Err(e) = wallet.persist(&mut conn) {
                                                                                let error_msg = format!("Failed to persist wallet: {}", e);
                                                                                if let Some(window) = app_handle.get_webview_window("main") {
                                                                                    let _ = window.emit("wallet-error", error_msg);
                                                                                }
                                                                                continue;
                                                                            }
                                                                            
                                                                            let balance = wallet.balance();
                                                                            let balance_info = format!("{}", balance.total().to_sat());
                                                                            
                                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                                let _ = window.emit("sync-completed", balance_info);
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            let error_msg = format!("Failed to apply update: {}", e);
                                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                                let _ = window.emit("wallet-error", error_msg);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    let error_msg = format!("Failed to sync: {}", e);
                                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                                        let _ = window.emit("wallet-error", error_msg);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let error_msg = format!("Failed to create esplora client: {}", e);
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("wallet-error", error_msg);
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let error_msg = format!("Failed to load wallet: {}", e);
                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                        let _ = window.emit("wallet-error", error_msg);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to open wallet database: {}", e);
                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                let _ = window.emit("wallet-error", error_msg);
                                            }
                                        }
                                    }
                                },
                                AppMessage::GetWalletBalance => {
                                    println!("Getting wallet balance");
                                    
                                    // Open the wallet database
                                    match Connection::open(DB_PATH) {
                                        Ok(mut conn) => {
                                            // Try to load the wallet
                                            let wallet_result = Wallet::load()
                                                .descriptor(KeychainKind::External, Some(EXTERNAL_DESC))
                                                .descriptor(KeychainKind::Internal, Some(INTERNAL_DESC))
                                                .extract_keys()
                                                .check_network(NETWORK)
                                                .load_wallet(&mut conn);
                                                
                                            match wallet_result {
                                                Ok(wallet_opt) => {
                                                    match wallet_opt {
                                                        Some(wallet) => {
                                                            let balance = wallet.balance();
                                                            let balance_info = format!("{}", balance.total().to_sat());
                                                            
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("wallet-balance", balance_info);
                                                            }
                                                        }
                                                        None => {
                                                            let error_msg = "Wallet not found. Create a wallet first.";
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("wallet-error", error_msg);
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let error_msg = format!("Failed to load wallet: {}", e);
                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                        let _ = window.emit("wallet-error", error_msg);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to open wallet database: {}", e);
                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                let _ = window.emit("wallet-error", error_msg);
                                            }
                                        }
                                    }
                                },
                                AppMessage::SendTransaction(amount) => {
                                    println!("Sending transaction of {} sats", amount);
                                    
                                    // Open the wallet database
                                    match Connection::open(DB_PATH) {
                                        Ok(mut conn) => {
                                            // Try to load the wallet
                                            let wallet_result = Wallet::load()
                                                .descriptor(KeychainKind::External, Some(EXTERNAL_DESC))
                                                .descriptor(KeychainKind::Internal, Some(INTERNAL_DESC))
                                                .extract_keys()
                                                .check_network(NETWORK)
                                                .load_wallet(&mut conn);
                                                
                                            match wallet_result {
                                                Ok(wallet_opt) => {
                                                    let mut wallet = match wallet_opt {
                                                        Some(wallet) => wallet,
                                                        None => {
                                                            let error_msg = "Wallet not found. Create a wallet first.";
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("wallet-error", error_msg);
                                                            }
                                                            continue;
                                                        }
                                                    };
                                                    
                                                    // Get the next unused address for receiving
                                                    let address = wallet.next_unused_address(KeychainKind::External);
                                                    
                                                    // Check if we have enough balance
                                                    let balance = wallet.balance();
                                                    let send_amount = Amount::from_sat(amount);
                                                    
                                                    if balance.total() < send_amount {
                                                        let error_msg = format!("Not enough funds. Required: {}, Available: {}", 
                                                            send_amount, balance.total());
                                                        if let Some(window) = app_handle.get_webview_window("main") {
                                                            let _ = window.emit("wallet-error", error_msg);
                                                        }
                                                        continue;
                                                    }
                                                    
                                                    // Build the transaction
                                                    let mut tx_builder = wallet.build_tx();
                                                    
                                                    // Add recipient - this modifies tx_builder in place
                                                    tx_builder.add_recipient(address.script_pubkey(), send_amount);
                                                    
                                                    // Finish building transaction
                                                    match tx_builder.finish() {
                                                        Ok(mut psbt) => {
                                                            match wallet.sign(&mut psbt, SignOptions::default()) {
                                                                Ok(finalized) => {
                                                                    if finalized {
                                                                        match psbt.extract_tx() {
                                                                            Ok(tx) => {
                                                                                // Create esplora client
                                                                                let client_result = esplora_client::Builder::new(ESPLORA_URL).build_async();
                                                                                match client_result {
                                                                                    Ok(client) => {
                                                                                        match client.broadcast(&tx).await {
                                                                                            Ok(_) => {
                                                                                                let txid = tx.compute_txid().to_string();
                                                                                                
                                                                                                if let Some(window) = app_handle.get_webview_window("main") {
                                                                                                    let _ = window.emit("transaction-sent", txid);
                                                                                                }
                                                                                            }
                                                                                            Err(e) => {
                                                                                                let error_msg = format!("Failed to broadcast transaction: {}", e);
                                                                                                if let Some(window) = app_handle.get_webview_window("main") {
                                                                                                    let _ = window.emit("wallet-error", error_msg);
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    Err(e) => {
                                                                                        let error_msg = format!("Failed to create esplora client: {}", e);
                                                                                        if let Some(window) = app_handle.get_webview_window("main") {
                                                                                            let _ = window.emit("wallet-error", error_msg);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            Err(e) => {
                                                                                let error_msg = format!("Failed to extract transaction: {}", e);
                                                                                if let Some(window) = app_handle.get_webview_window("main") {
                                                                                    let _ = window.emit("wallet-error", error_msg);
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        let error_msg = "Failed to finalize transaction";
                                                                        if let Some(window) = app_handle.get_webview_window("main") {
                                                                            let _ = window.emit("wallet-error", error_msg);
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    let error_msg = format!("Failed to sign transaction: {}", e);
                                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                                        let _ = window.emit("wallet-error", error_msg);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let error_msg = format!("Failed to build transaction: {}", e);
                                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                                let _ = window.emit("wallet-error", error_msg);
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let error_msg = format!("Failed to load wallet: {}", e);
                                                    if let Some(window) = app_handle.get_webview_window("main") {
                                                        let _ = window.emit("wallet-error", error_msg);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Failed to open wallet database: {}", e);
                                            if let Some(window) = app_handle.get_webview_window("main") {
                                                let _ = window.emit("wallet-error", error_msg);
                                            }
                                        }
                                    }
                                },
                            }
                        }
                        _ = sleep(Duration::from_secs(10)) => {
                            // Increment heartbeat counter
                            heartbeat_count += 1;
                            println!("Background task heartbeat: {}", heartbeat_count);
                            
                            // Send heartbeat event with counter value
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("heartbeat", heartbeat_count);
                            }
                        }
                    }
                }
                
                println!("Background task ended");
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, send_to_background])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
