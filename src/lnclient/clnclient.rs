use async_trait::async_trait;

use crate::lightning::parse_podcast_tlv;
use crate::lnclient::{LNClient, NodeInfo, Invoice, Payment};

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

impl CLNClient {

    pub async fn connect(url: String, cert_path: String, key_path: String, cacert_path: String) -> Result<CLNClient, Box<dyn Error>> {

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

    pub fn parse_payment(&mut self, cln_payment: cln::ListsendpaysPayments) -> Payment {
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
            custom_records: HashMap::new(),
            boostagram: None,
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

}

#[async_trait]
impl LNClient for CLNClient {

    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error>> {
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


    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error>> {
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

    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error>> {
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
            let payment_hash = HEXLOWER.encode(&cln_invoice.payment_hash);

            let payment_preimage = match cln_invoice.payment_preimage {
                Some(preimage) => HEXLOWER.encode(&preimage),
                None => "".to_string(),
            };

            let paid_at: i64 = cln_invoice.paid_at.unwrap_or_default().try_into().unwrap(); // convert u64 into i64

            let mut invoice = Invoice {
                index: cln_invoice.created_index.unwrap_or_default(),
                time: paid_at,
                payment_hash: payment_hash,
                preimage: payment_preimage,
                amount: 0,
                custom_records: HashMap::new(),
                boostagram: None,
            };

            if let Some(amt) = cln_invoice.amount_received_msat {
                let sats = amt.msat / 1000;
                invoice.amount = sats.try_into().unwrap(); // convert u64 into i64
            }
            else if let Some(amt) = cln_invoice.amount_msat {
                let sats = amt.msat / 1000;
                invoice.amount = sats.try_into().unwrap(); // convert u64 into i64
            }

            // CLN stuffs TLVs into the description field
            if let Some(desc) = cln_invoice.description {
                if desc.starts_with("keysend: {") {
                    // grab everything after 'keysend: '
                    let mut chars = desc[9..].chars();
                    let mut val = String::new();

                    // remove escaping around the tlv json
                    while let Some(c) = chars.next() {
                        val.push(match c {
                            '\\' => chars.next().unwrap_or_default(),
                            c => c,
                        });
                    }

                    // attempt to parse as podcast tlv
                    invoice.boostagram = Some(parse_podcast_tlv(&val.into()));
                }
            }

            invoices.push(invoice);
        }

        Ok(invoices)
    }


    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error>> {
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
            payments.push(self.parse_payment(cln_payment));
        }

        Ok(payments)
    }

    async fn keysend(&mut self, destination: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error>> {
        // let mut extratlvs: HashMap<u64, String> = HashMap::new();

        let mut extratlvs: Vec<cln::TlvEntry> = Vec::new();

        for (idx, val) in custom_records {
            extratlvs.push(cln::TlvEntry {
                r#type: idx,
                value: val,
            });
        }

        let destination = HEXLOWER.decode(destination.as_bytes())?;

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
            return Ok(self.parse_payment(cln_payment));
        }

        Err("Unable to find keysend payment".into())
    }

}