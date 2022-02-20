use std::error::Error;
use lnd;

pub fn connect(node_address: String, cert: String, macaroon: String) {
    let mut lightning;
    match lnd::Lnd::connect_with_macaroon(node_address, cert, macaroon).await {
        Ok(lndconn) => {
            println!(" - Success.");
            lightning = lndconn;
        }
        Err(e) => {
            println!("Could not connect to: [{}] using tls: [{}] and macaroon: [{}]", node_address, cert_path, macaroon_path);
            eprintln!("{:#?}", e);
            std::process::exit(1);
        }
    }
    return lightning;
}
