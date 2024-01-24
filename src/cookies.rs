use hyper::{Body, Request};
use hyper::header;
use cookie::{Cookie, CookieJar};

// copied from https://dzx.fr/blog/how-to-handle-http-requests-with-rust-and-hyper/#cookies
pub trait CookiesExt {
    fn cookies(&self) -> CookieJar;
}

impl CookiesExt for Request<Body> {
    fn cookies(&self) -> CookieJar {
        let mut jar = CookieJar::new();

        // Iterate on the Cookie header instances.
        for value in self.headers().get_all(header::COOKIE) {
            // Get the name-value pairs separated by semicolons.
            let it = match value.to_str() {
                Ok(s) => s.split(';').map(str::trim),
                Err(_) => continue,
            };

            // Iterate on the pairs.
            for s in it {
                // Parse and add the cookie to the jar.
                if let Ok(c) = Cookie::parse(s.to_owned()) {
                    jar.add_original(c);
                }
            }
        }

        jar
    }
}