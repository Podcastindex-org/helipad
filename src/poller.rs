use crate::HelipadConfig;
use crate::podcastindex;
use crate::boost;
use helipad_lnclient::LNClient;
use crate::lightning;
use crate::triggers;
use crate::WebSocketEvent;
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{anyhow, Result};
use futures::StreamExt;

use crate::REMOTE_GUID_CACHE_SIZE;

#[derive(Debug)]
enum BalanceError {
    TransportError,
    OtherError,
}

async fn poll_node_info(lightning: &mut dyn LNClient, db_filepath: &str) -> Result<()> {
    let node_info = lightning.get_info().await
        .map_err(|e| anyhow!("Error getting node info: {:#?}", e))?;

    println!("Node info: {:#?}", node_info);

    if let Err(e) = dbif::add_node_info_to_db(db_filepath, dbif::NodeInfoRecord {
        lnd_alias: node_info.alias,
        node_pubkey: node_info.pubkey,
        node_version: node_info.version,
    }) {
        eprintln!("Error adding node info to database: {:#?}", e);
    }

    Ok(())
}

async fn poll_balance(
    lightning: &mut dyn LNClient,
    db_filepath: &str,
    current_balance: &mut i64,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
) -> Result<(), BalanceError> {
    let new_balance = lightning.channel_balance().await.map_err(|e| {
        eprintln!("Error getting node balance: {:#?}", e);
        if e.to_string().contains("transport error") {
            BalanceError::TransportError
        } else {
            BalanceError::OtherError
        }
    })?;

    if dbif::add_wallet_balance_to_db(db_filepath, new_balance).is_err() {
        println!("Error adding wallet balance to the database.");
    }

    if *current_balance != new_balance {
        if let Err(e) = ws_tx.send(WebSocketEvent("balance".to_string(), serde_json::to_value(&new_balance).unwrap())) {
            eprintln!("Error sending WebSocket event: {:#?}", e);
        } else {
            println!("WebSocket event sent.");
        }
    }

    *current_balance = new_balance;
    Ok(())
}

async fn poll_invoices(
    lightning: &mut dyn LNClient,
    db_filepath: &str,
    current_index: &mut u64,
    remote_cache: &mut podcastindex::GuidCache,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
) {
    loop {
        let invoices = match lightning.list_invoices(*current_index, 1000).await {
            Ok(r) => r,
            Err(e) => { eprintln!("list_invoices failed: {}", e); return; }
        };

        let mut updated = false;
        for invoice in invoices {
            let invoice_index = invoice.index;
            let settled = invoice.settled;

            println!("Invoice: {}, hash: {}", invoice_index, invoice.payment_hash);

            if let Some(boost) = boost::parse_boost_from_invoice(invoice, remote_cache, false, "").await {
                println!("Boost: {:#?}", &boost);
                handle_boost(&boost, &db_filepath, &ws_tx, true).await;
            }
            else if settled {
                println!("No boost found for invoice: {}", invoice_index);
            }

            *current_index = invoice_index;
            updated = true;
        }

        if !updated {
            break;
        }
    }
}

async fn poll_payments(
    lightning: &mut dyn LNClient,
    db_filepath: &str,
    current_index: &mut u64,
    remote_cache: &mut podcastindex::GuidCache,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
    catchup: bool,
) -> bool {
    let payments = match lightning.list_payments(*current_index, 500).await {
        Ok(r) => r,
        Err(e) => { eprintln!("list_payments failed: {:#?}", e); return false; }
    };

    let mut updated = false;
    for payment in payments {
        let payment_index = payment.index;

        if let Some(boost) = boost::parse_boost_from_payment(payment.clone(), remote_cache).await {
            println!("Sent Boost: {:#?}", boost);
            handle_boost(&boost, db_filepath, ws_tx, !catchup).await;
        }

        *current_index = payment_index;
        updated = true;
    }
    updated
}

async fn handle_boost(
    boost: &dbif::BoostRecord,
    db_filepath: &str,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
    add_triggers: bool,
) {
    let ws_type = if boost.payment_info.is_some() {
        "payment".to_string()
    } else {
        boost.list_type()
    };

    if ws_type == "payment" {
        match dbif::add_payment_to_db(db_filepath, boost) {
            Ok(_) => println!("New payment added."),
            Err(e) => eprintln!("Error adding payment: {:#?}", e),
        }
    } else {
        match dbif::add_invoice_to_db(db_filepath, boost) {
            Ok(_) => println!("New invoice added."),
            Err(e) => eprintln!("Error adding invoice: {:#?}", e),
        }
    }

    let boost_with_effects = if add_triggers {
        triggers::process_triggers(db_filepath, boost).await.unwrap_or_else(|e| {
            eprintln!("Error processing triggers: {:#?}", e);
            triggers::BoostWithEffects { boost: boost.clone(), effects: Vec::new(), server_effects: Vec::new() }
        })
    } else {
        triggers::BoostWithEffects { boost: boost.clone(), effects: Vec::new(), server_effects: Vec::new() }
    };

    match ws_tx.send(WebSocketEvent(ws_type, serde_json::to_value(&boost_with_effects).unwrap())) {
        Ok(_) => println!("WebSocket event sent."),
        Err(e) => eprintln!("Error sending WebSocket event: {:#?}", e),
    }
}

pub async fn lnd_subscribe_invoices(
    helipad_config: HelipadConfig,
    ws_tx: Arc<broadcast::Sender<WebSocketEvent>>,
    settings: Arc<RwLock<dbif::SettingsRecord>>,
) {
    let db_filepath = helipad_config.database_file_path.clone();

    println!("\nConnecting to lightning node address...");
    let mut lightning = match lightning::connect(&helipad_config.lightning).await {
        Some(lnclient) => lnclient,
        None => {
            eprintln!("Error connecting to lightning node.");
            return;
        }
    };

    let mut remote_cache = podcastindex::GuidCache::new(REMOTE_GUID_CACHE_SIZE);
    let mut current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();

    println!("Getting existing invoices from node...");
    poll_invoices(&mut *lightning, &db_filepath, &mut current_index, &mut remote_cache, &ws_tx).await;

    println!("Current invoice index: {}", current_index);

    loop {
        println!("Subscribing to invoices starting at index: {}", current_index);

        let subscribe_result = lightning.subscribe_invoices(current_index).await;
        let mut retry_ms: u64 = 5000;

        let invoices = match subscribe_result {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Error subscribing to invoices: {:#?}", e);
                retry_ms = 60000;
                None
            }
        };

        if let Some(mut invoices) = invoices {
            while let Some(Ok(invoice)) = invoices.next().await {
                let invoice_index = invoice.index;
                let hash = invoice.payment_hash.clone();
                let settled = invoice.settled;

                println!("Invoice: {}, hash: {}", invoice_index, hash);

            let settings_snapshot = settings.read().await;
            let fetch_metadata = settings_snapshot.fetch_metadata;
            let metadata_whitelist = settings_snapshot.metadata_whitelist.clone();
            drop(settings_snapshot);

            if let Some(boost) = boost::parse_boost_from_invoice(invoice.clone(), &mut remote_cache, fetch_metadata, &metadata_whitelist).await {
                println!("Boost: {:#?}", &boost);
                handle_boost(&boost, &db_filepath, &ws_tx, true).await;
            }
            else if invoice.state == InvoiceState::Settled as i32 {
                println!("No boost found for invoice: {:#?}", &invoice);
            }

            eprintln!("Invoice subscription stream ended. Attempting to reconnect...");
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(retry_ms)).await;
    }
}

pub async fn lnd_poller(helipad_config: HelipadConfig, ws_tx: Arc<broadcast::Sender<WebSocketEvent>>) {
    let db_filepath = helipad_config.database_file_path.clone();

    println!("\nConnecting to lightning node address...");
    let mut lightning = lightning::connect_or_exit(&helipad_config.lightning).await;

    if let Err(e) = poll_node_info(&mut *lightning, &db_filepath).await {
        eprintln!("Error updating node info: {:#?}", e);
    }

    let mut remote_cache = podcastindex::GuidCache::new(REMOTE_GUID_CACHE_SIZE);
    let mut current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();
    let mut current_balance = 0i64;
    let mut catchup = true;

    loop {
        if let Err(e) = poll_balance(&mut *lightning, &db_filepath, &mut current_balance, &ws_tx).await {
            eprintln!("Error polling balance: {:#?}", e);
            if let BalanceError::TransportError = e {
                if let Some(conn) = lightning::connect(&helipad_config.lightning).await {
                    println!(" - Reconnected.");
                    lightning = conn;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(9000)).await;
            continue;
        }

        let updated = poll_payments(&mut lightning, &db_filepath, &mut current_payment, &mut remote_cache, &ws_tx, catchup).await;
        println!("Current payment: {}", current_payment);

        if !updated {
            tokio::time::sleep(tokio::time::Duration::from_millis(9000)).await;
            catchup = false;
        }
    }
}