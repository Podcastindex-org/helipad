# Helipad
This package will poll a Lightning LND node for invoices related to Podcasting 2.0 and display them in a web interface.  It's
intended for use as a way to see incoming Boost-a-grams and other data from podcast listeners in a browser.

Helipad is intended to run as a dockerized Umbrel app, but can also be run standalone.

Helipad runs as a single process web server with the LND poller running as a separate thread.  Invoices are checked for every
9 seconds, parsed and stored locally in a Sqlite database.  The main webserver thread then serves them to clients over HTTP(S).

## To Compile

You need Rust installed: [instructions here](https://www.rust-lang.org/tools/install).

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

Once you have Rust you can compile in the root folder of this repo:

`cargo run`

If you get an error like:

`  = note: /usr/bin/ld: cannot find -lsqlite3`

You may need to install SQLite3:

`sudo apt-get install sqlite3`

.... but this still doesn't work.

## After compiling

After compiling, you start the binary like this:

```./helipad 8080```

You may pass the port number you want it to listen on, on the command line as the only argument.  If you don't pass a port number
it will listen by default on port 2112. (RIP Neil)

The FQDN of your LND node must be present in an environment variable called $LND_URL in order to connect to it, like this:

```export LND_URL="mynode.example.com:10009"```

If you don't export that variable, it will attempt to connect to "localhost:10009".

Helipad also needs your admin.macaroon and tls.cert files.  It will first look for them in the standard LND locations.  If it cannot
find them there, it will fall back to looking for them both in the local working directory that Helipad is running from.

Information about the Umbrel app environment is in the umbrel folder for those interested.