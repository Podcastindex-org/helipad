# Helipad
This package will poll a Lightning LND node for invoices related to Podcasting 2.0 and display them in a web interface.  It's
intended for use as a way to see incoming Boost-a-grams and other data from podcast listeners in a browser.

Helipad is intended to run as a dockerized Umbrel app, but can also be run as a standalone executable if compiled from source.

Helipad runs as a single process web server with the LND poller running in a separate thread.  Invoices are checked for every
9 seconds, parsed and stored locally in a Sqlite database.  The main webserver thread then serves them to clients over HTTP(S).

After compiling, you start the binary like this:

```./helipad 8080```

You may pass the port number you want it to listen on, on the command line as the only argument.  If you don't pass a port number
it will listen by default on port 2112. (RIP Neil)

The FQDN of your LND node must be present in an environment variable called $LND_URL in order to connect to it, like this:

```export LND_URL="mynode.example.com:10009"```

If you don't export that variable, it will attempt to connect to "localhost:10009".

Helipad also needs your admin.macaroon and tls.cert files.  It will first look for them in the locations pointed to by these two
environment variables:

 - LND_ADMINMACAROON
 - LND_TLSCERT

Information about the Umbrel app environment is in the umbrel folder for those interested.


## Configuration
Each configurable item has multiple options.  They are listed in the config file [here](helipad.conf).  In each case, the environment
variable is tried first, then the configuration file parameter, then a sane default based on known locations in use from other
projects.

The only exception to this is the `listen_port` which can be specified on the command line as the only argument.  This is just for
convenience as it's a very common thing to change during testing.


## API
The very simplistic API consists of the following endpoints:

#### /api/v1/index
This call returns the current most recent invoice index number that Helipad has reconciled with LND.

#### /api/v1/balance
This call returns the current channel balance that LND is reporting.

#### /api/v1/boosts
This call returns `count` boosts starting at `index`.  If the `old` parameter is present, the boosts returned start from `index` and
descend by `count`, showing older boosts.  Otherwise, they start at `index` and ascend by `count`, showing newer boosts.

#### /api/v1/streams
This call returns `count` streams starting at `index`.  If the `old` parameter is present, the streams returned start from `index` and
descend by `count`, showing older streams.  Otherwise, they start at `index` and ascend by `count`, showing newer streams.


## CSV export
There is an endpoint called `/csv` that will export boosts as a CSV list to make organizing easier.  The parameters behave just like the
`boosts` and `streams` endpoints, but also accept an `end` parameter which limits how far back in time the CSV export list goes.  An example
call would look like this:

```http
GET http://localhost:2112/csv?index=38097&count=13049&old=true&end=25048
```

This will give back a csv list of 13,049 boosts starting at index 38097 and descending back in time - but will not go past index number
25048.


## Development

### Quick Start (Debian/Linux Mint/Ubuntu/Pop!_OS) 

In order to run Helipad locally you need to install the Rust Compiler `rustc`, the Rust package manager `cargo`, and the needed shared libraries `libssl-dev`/`libsqlite3-dev`. Clone the Github repo with `git clone ...` and enter the `helipad` directory. Note, all commands going forward will need to be ran from this directory. `cargo run` will compile and run helipad. If Helipad fails to start you may need to edit `helipad.conf` or set/unset Environment Variables.

```sh
sudo apt install rustc cargo libssl-dev libsqlite3-dev
git clone https://github.com/Podcastindex-org/helipad.git
cd helipad
# Edit helipad.conf as needed
cargo run
# Open http://127.0.0.1:2112 in your browser
```
