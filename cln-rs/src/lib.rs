pub mod cln;

// use cln::node_client;
// use cln::{listinvoices_invoices::ListinvoicesInvoicesStatus, node_client::NodeClient};
// use cln::{GetinfoRequest, InvoiceRequest, PayRequest};

// use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};

// use std::fs;
// use std::error::Error;

// pub struct CLNClient {
//     url: String,
//     client: NodeClient<Channel>,
// }

// impl CLNClient {

//     pub async fn connect(url: String, cert_path: String, key_path: String, cacert_path: String) -> Result<CLNClient, Box<dyn Error>> {

//         let cert_text: Vec<u8> = fs::read(cert_path.clone())?;
//         let key_text: Vec<u8> = fs::read(key_path.clone())?;
//         let cacert_text: Vec<u8> = fs::read(cacert_path.clone())?;

//         let ca_certificate = Certificate::from_pem(&cacert_text);
//         let client_identity = Identity::from_pem(&cert_text, &key_text);


//     let tls = ClientTlsConfig::new()
//         .ca_certificate(ca_certificate)
//         .domain_name("example.com");

//     let channel = Channel::from_static("https://[::1]:50051")
//         .tls_config(tls)?
//         .connect()
//         .await?;

//     let client = NodeClient::new(channel);

//         // let tls_config = ClientTlsConfig::new()
//         //     .domain_name("localhost")
//         //     .ca_certificate(ca_certificate)
//         //     .identity(client_identity);

//         // let channel = Channel::from_shared(url)?
//         //     .tls_config(tls_config)?
//         //     .connect()
//         //     .await?;

//         // let client = NodeClient::new(channel);

//         Ok(CLNClient {
//             url,
//             client
//         })
//     }
// }


// // pub fn add(left: usize, right: usize) -> usize {
// //     left + right
// // }

// // #[cfg(test)]
// // mod tests {
// //     use super::*;

// //     #[test]
// //     fn it_works() {
// //         let result = add(2, 2);
// //         assert_eq!(result, 4);
// //     }
// // }

