pub mod lnd {
    pub mod lnrpc {
        tonic::include_proto!("lnrpc");
    }
    pub mod routerrpc {
        tonic::include_proto!("routerrpc");
    }
}

pub mod cln {
    tonic::include_proto!("cln");
}