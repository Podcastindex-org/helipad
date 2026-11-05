use async_trait::async_trait;

use crate::{Invoice, InvoiceStream, LNClient, LNClientError, NodeInfo, Payment};

use data_encoding::HEXLOWER;

use futures::StreamExt;

use rand::RngExt;

use sha2::{Sha256, Digest};

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use std::vec::Vec;

use crate::proto::lnd::lnrpc::{
    ChannelBalanceRequest, GetInfoRequest, Invoice as LndInvoice, InvoiceSubscription,
    ListInvoiceRequest, ListPaymentsRequest, Payment as LndPayment,
    lightning_client::LightningClient,
};

use crate::proto::lnd::lnrpc::invoice::InvoiceState;
use crate::proto::lnd::lnrpc::payment::PaymentStatus;
use crate::proto::lnd::routerrpc::{SendPaymentRequest, router_client::RouterClient};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};

use tonic::service::{interceptor::InterceptedService, Interceptor};
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::{Request, Status};

type AuthenticatedLightningClient = LightningClient<InterceptedService<Channel, MacaroonAuth>>;
type AuthenticatedRouterClient = RouterClient<InterceptedService<Channel, MacaroonAuth>>;

pub const TLV_KEYSEND: u64 = 5482373484;

/// Attaches the hex-encoded macaroon to every gRPC request.
#[derive(Clone)]
struct MacaroonAuth {
    macaroon: String,
}

impl Interceptor for MacaroonAuth {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let macaroon = self.macaroon.parse().map_err(|_| {
            Status::invalid_argument("unable to convert macaroon to metadata value")
        })?;

        request.metadata_mut().insert("macaroon", macaroon);

        Ok(request)
    }
}

/// Trusts only the exact certificate bytes from LND's tls.cert.
///
/// The hostname in the connection URL is not checked, so a node reached by
/// an IP or a name missing from the certificate's SAN still connects.
#[derive(Debug)]
struct PinnedCertVerifier {
    pinned: Vec<CertificateDer<'static>>,
    provider: rustls::crypto::CryptoProvider,
}

impl ServerCertVerifier for PinnedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if self.pinned.iter().any(|cert| cert.as_ref() == end_entity.as_ref()) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

pub struct LNDClient {
    client: AuthenticatedLightningClient,
    router: AuthenticatedRouterClient,
}

#[async_trait]
impl LNClient for LNDClient {

    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error + Send + Sync>> {
        let info = self.client.get_info(GetInfoRequest {}).await?.into_inner();

        Ok(NodeInfo {
            pubkey: info.identity_pubkey,
            alias: info.alias,
            version: info.version,
            nodetype: "LND".to_string(),
        })
    }

    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let balance = self.client.channel_balance(ChannelBalanceRequest {}).await?.into_inner();
        let mut current_balance: i64 = 0;

        if let Some(bal) = balance.local_balance {
            current_balance = bal.sat as i64;
        }

        Ok(current_balance)
    }

    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error + Send + Sync>> {
        let request = ListInvoiceRequest {
            index_offset: start,
            num_max_invoices: limit,
            ..Default::default()
        };

        let result = match self.client.list_invoices(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => {
                return Err(format!("unable to fetch invoices: {status}").into());
            }
        };

        let mut invoices: Vec<Invoice> = Vec::new();

        for item in result.invoices {
            invoices.push(parse_invoice(item));
        }


        Ok(invoices)
    }

    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error + Send + Sync>> {
        let request = ListPaymentsRequest {
            index_offset: start,
            max_payments: limit,
            ..Default::default()
        };

        let result = match self.client.list_payments(request).await {
            Ok(response) => response.into_inner(),
            Err(status) => {
                return Err(format!("unable to fetch payments: {status}").into());
            }
        };

        let mut payments: Vec<Payment> = Vec::new();

        for item in result.payments {
            payments.push(parse_payment(&item));
        }

        Ok(payments)
    }

    async fn subscribe_invoices(&mut self, start_index: u64) -> Result<InvoiceStream, Box<dyn Error + Send + Sync>> {
        let mut client = self.client.clone();

        let request = InvoiceSubscription {
            add_index: start_index,
            settle_index: 0,
        };

        let invoices = client.subscribe_invoices(request).await?.into_inner();

        let stream = invoices.map(|item| match item {
            Ok(invoice) => Ok(parse_invoice(invoice)),
            Err(status) => Err(LNClientError(status.to_string())),
        });

        Ok(Box::pin(stream))
    }

    async fn send_keysend(&mut self, destination: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>> {
        // convert pub key hash to raw bytes
        let raw_pubkey = HEXLOWER.decode(destination.as_bytes())?;
    
        // generate 32 random bytes for pre_image
        let mut pre_image = [0u8; 32];
        {
            let mut rng = rand::rng();
            rng.fill(&mut pre_image);
        }
    
        // and convert to sha256 hash
        let payment_hash = Sha256::digest(pre_image);

        // add keysend tlv
        let mut dest_custom_records = custom_records.clone();
        dest_custom_records.insert(TLV_KEYSEND, pre_image.to_vec());

        let req = SendPaymentRequest {
            dest: raw_pubkey,
            amt: sats as i64,
            payment_hash: payment_hash.to_vec(),
            dest_custom_records,
            timeout_seconds: 60,
            ..Default::default()
        };

        send_payment(&mut self.router, req).await
    }

    async fn send_bolt11(&mut self, payment_request: String, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>> {
        let req = SendPaymentRequest {
            payment_request,
            dest_custom_records: custom_records,
            timeout_seconds: 60,
            ..Default::default()
        };

        send_payment(&mut self.router, req).await
    }
}

pub async fn connect(node_address: String, cert_path: String, macaroon_path: String) -> Result<LNDClient, Box<dyn Error + Send + Sync>> {
    let cert: Vec<u8> = match fs::read(cert_path.clone()) {
        Ok(cert) => cert,
        Err(e) => {
            return Err(Box::new(
                LNClientError(format!("Unable to open certificate file {}: {}", cert_path, e)))
            );
        },
    };

    let macaroon: Vec<u8> = match fs::read(macaroon_path.clone()) {
        Ok(macaroon) => macaroon,
        Err(e) => {
            return Err(Box::new(
                LNClientError(format!("Unable to open macaroon file {}: {}", macaroon_path, e)))
            );
        },
    };

    let pinned = match CertificateDer::pem_slice_iter(&cert).collect::<Result<Vec<_>, _>>() {
        Ok(certs) if !certs.is_empty() => certs,
        Ok(_) => {
            return Err(Box::new(LNClientError(format!(
                "Certificate file {} contains no PEM certificates",
                cert_path
            ))));
        }
        Err(e) => {
            return Err(Box::new(LNClientError(format!(
                "Unable to parse certificate file {}: {}",
                cert_path, e
            ))));
        }
    };

    // LND serves gRPC over TLS with a self-signed certificate. Trust only the
    // exact tls.cert bytes and skip the hostname check.
    let verifier = PinnedCertVerifier {
        pinned,
        provider: rustls::crypto::aws_lc_rs::default_provider(),
    };

    let channel = Channel::from_shared(node_address.clone())?
        .tls_config_with_verifier(ClientTlsConfig::new(), Arc::new(verifier))?
        .connect()
        .await?;

    // LND expects a hex-encoded macaroon in the "macaroon" metadata header
    let auth = MacaroonAuth {
        macaroon: HEXLOWER.encode(&macaroon),
    };

    let client = LightningClient::with_interceptor(channel.clone(), auth.clone());
    let router = RouterClient::with_interceptor(channel, auth);

    Ok(LNDClient {
        client,
        router,
    })
}

fn parse_payment(item: &LndPayment) -> Payment {
    let mut pay = Payment {
        index: item.payment_index,
        time: item.creation_time_ns / 1000000000,
        amount: item.value_sat,

        destination: String::new(), // hop pubkey
        payment_hash: item.payment_hash.clone(),
        payment_preimage: item.payment_preimage.clone(),

        fee: item.fee_sat,

        custom_records: HashMap::new(),
    };

    for htlc in &item.htlcs {
        let Some(route) = htlc.route.as_ref() else {
            continue; // no route found
        };
        let Some(hop) = route.hops.last() else {
            continue; // route with no hops
        };

        pay.destination = hop.pub_key.clone();

        for (key, value) in &hop.custom_records {
            pay.custom_records.insert(*key, value.clone());
        }
    }

    pay
}

fn parse_invoice(item: LndInvoice) -> Invoice {
    let payment_hash = HEXLOWER.encode(&item.r_hash);
    let preimage = HEXLOWER.encode(&item.r_preimage);

    let mut inv = Invoice {
        index: item.add_index,
        time: item.settle_date,
        amount: item.amt_paid_sat,
        memo: item.memo,
        payment_hash,
        preimage,
        custom_records: HashMap::new(),
        settled: item.state == InvoiceState::Settled as i32,
    };

    for htlc in item.htlcs {
        for (key, value) in &htlc.custom_records {
            inv.custom_records.insert(*key, value.clone());
        }
    }

    inv
}

async fn send_payment(client: &mut AuthenticatedRouterClient, payment_request: SendPaymentRequest) -> Result<Payment, Box<dyn Error + Send + Sync>> {
    println!("Sending payment to: {:#?}", payment_request);

    // send payment using send_payment_v2 and get payment stream
    let mut payment_stream = client.send_payment_v2(payment_request).await?.into_inner();

    // wait for payment to succeed or fail
    while let Some(payment_update) = payment_stream.message().await? {
        if payment_update.status == PaymentStatus::Succeeded as i32 {
            return Ok(parse_payment(&payment_update));
        }
        else if payment_update.status == PaymentStatus::Failed as i32 {
            return Err(Box::new(LNClientError("Payment failed".into())));
        }
    }

    Err(Box::new(LNClientError("Payment timed out".into())))
}
