use async_trait::async_trait;

use crate::{Invoice, InvoiceStream, LNClient, LNClientError, NodeInfo, Payment};

use futures::stream;

use data_encoding::HEXLOWER;

use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::convert::TryInto;

use cln_rs::cln::node_client::NodeClient;
use cln_rs::cln;

pub struct CLNClient {
    client: NodeClient<Channel>,
}

#[async_trait]
impl LNClient for CLNClient {

    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error + Send + Sync>> {
        let request = cln::GetinfoRequest {};
        let response = self.client.getinfo(request).await?.into_inner();

        let info = NodeInfo {
            pubkey: HEXLOWER.encode(&response.id),
            alias: response.alias.unwrap_or_default(),
            version: response.version,
            nodetype: "CLN".to_string(),
        };

        Ok(info)
    }

    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let response = self.client.bkpr_list_balances(cln::BkprlistbalancesRequest {})
            .await?
            .into_inner();

        let mut local_balance: i64 = 0;

        for account in response.accounts {
            if account.account == "wallet" {
                continue; // skip onchain balance
            }

            if account.account_resolved.unwrap_or_default() {
                continue; // closed and resolved channel
            }

            for balance in account.balances {
                if let Some(balance) = balance.balance_msat {
                    let sats: i64 = (balance.msat / 1000).try_into().unwrap(); // convert u64 into i64
                    local_balance += sats
                }
            }
        }

        Ok(local_balance)
    }

    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error + Send + Sync>> {
        let limit: u32 = limit.try_into().unwrap(); // convert u64 into required u32

        let request = cln::ListinvoicesRequest {
            index: Some(cln::listinvoices_request::ListinvoicesIndex::Created.into()),
            start: Some(start + 1),
            limit: Some(limit),
            ..Default::default()
        };

        let response = self.client.list_invoices(request)
            .await?
            .into_inner();

        let mut invoices: Vec<Invoice> = Vec::new();

        for cln_invoice in response.invoices {
            let settled = cln_invoice.status
                == cln::listinvoices_invoices::ListinvoicesInvoicesStatus::Paid as i32;

            invoices.push(parse_cln_invoice(
                cln_invoice.created_index.unwrap_or_default(),
                cln_invoice.paid_at.unwrap_or_default(),
                &cln_invoice.payment_hash,
                cln_invoice.payment_preimage.as_deref(),
                cln_invoice.amount_received_msat.as_ref(),
                cln_invoice.amount_msat.as_ref(),
                cln_invoice.description.as_deref(),
                settled,
            ));
        }

        Ok(invoices)
    }

    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error + Send + Sync>> {
        let limit: u32 = limit.try_into().unwrap(); // convert u64 into u32

        let request = cln::ListsendpaysRequest {
            index: Some(cln::listsendpays_request::ListsendpaysIndex::Created.into()),
            start: Some(start + 1),
            limit: Some(limit),
            status: Some(cln::listsendpays_request::ListsendpaysStatus::Complete.into()),
            ..Default::default()
        };

        let response = self.client.list_send_pays(request)
            .await?
            .into_inner();

        let mut payments: Vec<Payment> = Vec::new();

        for cln_payment in response.payments {
            payments.push(parse_payment(cln_payment, &HashMap::new()));
        }

        Ok(payments)
    }

    async fn subscribe_invoices(&mut self, start_index: u64) -> Result<InvoiceStream, Box<dyn Error + Send + Sync>> {
        let client = self.client.clone();

        let stream = stream::unfold(Some((client, start_index)), |state| async move {
            let (mut client, last) = state?;

            let request = cln::WaitanyinvoiceRequest {
                lastpay_index: Some(last),
                timeout: None,
            };

            match client.wait_any_invoice(request).await {
                Ok(response) => {
                    let response = response.into_inner();
                    let Some(index) = response.pay_index else {
                        return Some((Err(LNClientError("invoice missing pay_index".into())), None));
                    };

                    let settled = response.status
                        == cln::waitanyinvoice_response::WaitanyinvoiceStatus::Paid as i32;
                    let invoice = parse_cln_invoice(
                        index,
                        response.paid_at.unwrap_or_default(),
                        &response.payment_hash,
                        response.payment_preimage.as_deref(),
                        response.amount_received_msat.as_ref(),
                        response.amount_msat.as_ref(),
                        Some(response.description.as_str()),
                        settled,
                    );

                    Some((Ok(invoice), Some((client, index))))
                }
                Err(status) => Some((Err(LNClientError(status.to_string())), None)),
            }
        });

        Ok(Box::pin(stream))
    }

    async fn send_keysend(&mut self, dest: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>> {
        // let mut extratlvs: HashMap<u64, String> = HashMap::new();

        let mut extratlvs: Vec<cln::TlvEntry> = Vec::new();
        for (idx, val) in &custom_records {
            extratlvs.push(cln::TlvEntry {
                r#type: *idx,
                value: val.clone(),
            });
        }

        let destination = HEXLOWER.decode(dest.as_bytes())?;

        // send keysend payment
        let request = cln::KeysendRequest {
            destination: destination,
            amount_msat: Some(cln::Amount {
                msat: sats * 1000
            }),
            extratlvs: Some(cln::TlvStream {
                entries: extratlvs,
            }),
            ..Default::default()
        };

        let response = match self.client.key_send(request).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Error sending keysend: {:#?}", e);
                return Err(format!("{}", e.message()).into());
            }
        };

        let response = response.into_inner();

        // look up full payment info in listsendpays
        let request = cln::ListsendpaysRequest {
            payment_hash: Some(response.payment_hash),
            ..Default::default()
        };

        let response = self.client.list_send_pays(request)
            .await?
            .into_inner();

        for cln_payment in response.payments {
            return Ok(parse_payment(cln_payment, &custom_records));
        }

        Err("Unable to find keysend payment".into())
    }

    async fn send_bolt11(&mut self, payment_request: String, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>> {
        // if !custom_records.is_empty() {
        //     eprintln!("Warning: CLN does not support attaching custom records to bolt11 payments, ignoring {} records", custom_records.len());
        // }

        // send bolt11 payment
        let request = cln::PayRequest {
            bolt11: payment_request,
            retry_for: Some(60),
            ..Default::default()
        };

        let response = match self.client.pay(request).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Error sending bolt11 payment: {:#?}", e);
                return Err(format!("{}", e.message()).into());
            }
        };

        let response = response.into_inner();

        if response.status != cln::pay_response::PayStatus::Complete as i32 {
            return Err("Payment failed".into());
        }

        // look up full payment info in listsendpays
        let request = cln::ListsendpaysRequest {
            payment_hash: Some(response.payment_hash),
            ..Default::default()
        };

        let response = self.client.list_send_pays(request)
            .await?
            .into_inner();

        for cln_payment in response.payments {
            return Ok(parse_payment(cln_payment, &custom_records));
        }

        Err("Unable to find bolt11 payment".into())
    }

}

pub async fn connect(url: String, cert_path: String, key_path: String, cacert_path: String) -> Result<CLNClient, Box<dyn Error + Send + Sync>> {
    let cert_text: Vec<u8> = fs::read(cert_path.clone())?;
    let key_text: Vec<u8> = fs::read(key_path.clone())?;
    let cacert_text: Vec<u8> = fs::read(cacert_path.clone())?;

    let ca_certificate = Certificate::from_pem(&cacert_text);
    let client_identity = Identity::from_pem(&cert_text, &key_text);

    let tls_config = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(ca_certificate)
        .identity(client_identity);

    let urlcopy = url.to_owned();

    let channel = Channel::from_shared(urlcopy)?
        .tls_config(tls_config)?
        .connect()
        .await?;

    let client = NodeClient::new(channel);

    Ok(CLNClient {
        client
    })
}

pub fn parse_payment(cln_payment: cln::ListsendpaysPayments, custom_records: &HashMap<u64, Vec<u8>>) -> Payment {
    let destination = cln_payment.destination.unwrap_or_default();
    let destination = HEXLOWER.encode(&destination);

    let payment_hash = HEXLOWER.encode(&cln_payment.payment_hash);

    let payment_preimage = cln_payment.payment_preimage.unwrap_or_default();
    let payment_preimage = HEXLOWER.encode(&payment_preimage);

    let mut pay = Payment {
        index: cln_payment.created_index.unwrap_or_default(),
        time: cln_payment.created_at.try_into().unwrap(), // convert u64 to i64
        amount: 0,
        destination: destination,
        payment_hash: payment_hash,
        payment_preimage: payment_preimage,
        fee: 0,
        custom_records: custom_records.clone(),
    };

    if let Some(amount_sent) = cln_payment.amount_sent_msat {
        let sats_sent: i64 = (amount_sent.msat / 1000).try_into().unwrap(); // convert u64 into i64
        pay.amount = sats_sent;
    }

    if let Some(amount_recv) = cln_payment.amount_msat {
        let sats_recv: i64 = (amount_recv.msat / 1000).try_into().unwrap(); // convert u64 into i64
        pay.fee = pay.amount - sats_recv;
    }

    return pay;
}

fn sats_from_amount(amount: Option<&cln::Amount>) -> Option<i64> {
    amount.map(|amount| (amount.msat / 1000).try_into().unwrap())
}

fn parse_cln_invoice(
    index: u64,
    paid_at: u64,
    payment_hash: &[u8],
    payment_preimage: Option<&[u8]>,
    amount_received_msat: Option<&cln::Amount>,
    amount_msat: Option<&cln::Amount>,
    description: Option<&str>,
    settled: bool,
) -> Invoice {
    Invoice {
        index,
        time: paid_at.try_into().unwrap(),
        payment_hash: HEXLOWER.encode(payment_hash),
        preimage: payment_preimage.map(|preimage| HEXLOWER.encode(preimage)).unwrap_or_default(),
        amount: sats_from_amount(amount_received_msat).or_else(|| sats_from_amount(amount_msat)).unwrap_or(0),
        memo: description.unwrap_or_default().to_string(),
        custom_records: HashMap::new(),
        settled,
    }
}
