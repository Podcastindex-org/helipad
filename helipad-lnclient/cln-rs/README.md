### Note

CLN provides a `cln-grpc` crate that provides the same definitions as this crate. I was not able to get CLN's crate to work, due to potential version
differences in tonic, prost, axum, or maybe another library we're using.

As a result, I've built this crate using the proto files extracted from the `cln-gprc` crate.
