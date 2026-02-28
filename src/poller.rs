use crate::HelipadConfig;
use crate::podcastindex;
use crate::boost;
use data_encoding::HEXLOWER;
use dbif;
use lnd::lnrpc::lnrpc::invoice::InvoiceState;
use crate::lightning;
use crate::triggers;
use crate::WebSocketEvent;
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, Context};
use futures::StreamExt;

use crate::REMOTE_GUID_CACHE_SIZE;

#[derive(Debug)]
enum BalanceError {
    TransportError,
    OtherError,
}

async fn poll_node_info(lightning: &mut lnd::Lnd, db_filepath: &str) -> Result<()> {
    let node_info = lnd::Lnd::get_info(lightning).await
        .with_context(|| "Error getting LND node info")?;

    println!("LND node info: {:#?}", node_info);

    if let Err(e) = dbif::add_node_info_to_db(db_filepath, dbif::NodeInfoRecord {
        lnd_alias: node_info.alias,
        node_pubkey: node_info.identity_pubkey,
        node_version: node_info.version,
    }) {
        eprintln!("Error adding node info to database: {:#?}", e);
    }

    Ok(())
}

async fn poll_balance(
    lightning: &mut lnd::Lnd,
    db_filepath: &str,
    current_balance: &mut i64,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
) -> Result<(), BalanceError> {
    let balance = lnd::Lnd::channel_balance(lightning).await.map_err(|status| {
        eprintln!("Error getting LND wallet balance: {:#?}", status);
        if status.message() == "transport error" {
            BalanceError::TransportError
        } else {
            BalanceError::OtherError
        }
    })?;

    let new_balance = balance.local_balance
        .map(|bal| { println!("LND node local balance: {:#?}", bal.sat); bal.sat as i64 })
        .unwrap_or(0);

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
    lightning: &mut lnd::Lnd,
    db_filepath: &str,
    current_index: &mut u64,
    remote_cache: &mut podcastindex::GuidCache,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
) -> bool {
    let response = match lnd::Lnd::list_invoices(lightning, false, *current_index, 500, false, 0, 0).await {
        Ok(r) => r,
        Err(e) => { eprintln!("lnd::Lnd::list_invoices failed: {}", e); return false; }
    };

    let mut updated = false;
    for invoice in response.invoices {
        let hash = HEXLOWER.encode(&invoice.r_hash);

        println!("Invoice: {}, state: {}, hash: {}", invoice.add_index, invoice.state, hash);

        if let Some(boost) = boost::parse_boost_from_invoice(invoice.clone(), remote_cache, false, "").await {
            println!("Boost: {:#?}", &boost);
            handle_boost(&boost, db_filepath, ws_tx, false).await;
        }
        else if invoice.state == InvoiceState::Settled as i32 {
            println!("No boost found for invoice: {:#?}", &invoice);
        }

        *current_index = invoice.add_index;
        updated = true;
    }
    updated
}

async fn poll_payments(
    lightning: &mut lnd::Lnd,
    db_filepath: &str,
    current_index: &mut u64,
    remote_cache: &mut podcastindex::GuidCache,
    ws_tx: &Arc<broadcast::Sender<WebSocketEvent>>,
    catchup: bool,
) -> bool {
    let response = match lnd::Lnd::list_payments(lightning, false, *current_index, 500, false, false, 0, 0).await {
        Ok(r) => r,
        Err(e) => { eprintln!("lnd::Lnd::list_payments failed: {:#?}", e); return false; }
    };

    let mut updated = false;
    for payment in response.payments {
        println!("Payment: {}, hash: {}", payment.payment_index, payment.payment_hash);

        if let Some(boost) = boost::parse_boost_from_payment(payment.clone(), remote_cache).await {
            println!("Sent Boost: {:#?}", boost);
            handle_boost(&boost, db_filepath, ws_tx, !catchup).await;
        }

        *current_index = payment.payment_index;
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

    println!("\nConnecting to LND node address...");
    let mut lightning = lightning::connect_lnd_or_exit(&helipad_config.node_address, &helipad_config.cert_path, &helipad_config.macaroon_path).await;
    let mut remote_cache = podcastindex::GuidCache::new(REMOTE_GUID_CACHE_SIZE);
    let mut current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();

    println!("Getting existing invoices from LND...");
    poll_invoices(&mut lightning, &db_filepath, &mut current_index, &mut remote_cache, &ws_tx).await;

    println!("Current invoice index: {}", current_index);

    loop {
        println!("Subscribing to LND invoices starting at index: {}", current_index);
        let invoices = lightning.subscribe_invoices(current_index, 0).await;

        let mut invoices = match invoices {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error subscribing to invoices: {:#?}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(60000)).await;
                continue;
            }
        };

        while let Some(Ok(invoice)) = invoices.next().await {
            let hash = HEXLOWER.encode(&invoice.r_hash);

            println!("Invoice: {}, state: {}, hash: {}", invoice.add_index, invoice.state, hash);

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

            current_index = invoice.add_index;
            println!("Current index: {}", current_index);
        }

        eprintln!("Invoice subscription stream ended. Attempting to reconnect...");
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
    }
}

pub async fn lnd_poller(helipad_config: HelipadConfig, ws_tx: Arc<broadcast::Sender<WebSocketEvent>>) {
    let db_filepath = helipad_config.database_file_path.clone();

    println!("\nConnecting to LND node address...");
    let mut lightning = lightning::connect_lnd_or_exit(&helipad_config.node_address, &helipad_config.cert_path, &helipad_config.macaroon_path).await;

    if let Err(e) = poll_node_info(&mut lightning, &db_filepath).await {
        eprintln!("Error updating node info: {:#?}", e);
    }

    let mut remote_cache = podcastindex::GuidCache::new(REMOTE_GUID_CACHE_SIZE);
    let mut current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();
    let mut current_balance = 0i64;
    let mut catchup = true;

    loop {
        if let Err(e) = poll_balance(&mut lightning, &db_filepath, &mut current_balance, &ws_tx).await {
            eprintln!("Error polling balance: {:#?}", e);
            if let BalanceError::TransportError = e {
                if let Some(conn) = lightning::connect_to_lnd(&helipad_config.node_address, &helipad_config.cert_path, &helipad_config.macaroon_path).await {
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