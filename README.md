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


<br><br>
## Configuration
Each configurable item has multiple options.  They are listed in the config file [here](helipad.conf).  In each case, the environment
variable is tried first, then the configuration file parameter, then a sane default based on known locations in use from other
projects.

The only exception to this is the `listen_port` which can be specified on the command line as the only argument.  This is just for
convenience as it's a very common thing to change during testing.


<br><br>
## API
The very simplistic API consists of the following endpoints:

#### /api/v1/login
This call accepts a JSON body with `password` and optional `stay_logged_in` fields and returns a JWT token for API authentication. Use the token in the `Authorization: Bearer <token>` header for API requests. Tokens expire after 1 hour by default or 30 days if `stay_logged_in` is included.

#### /api/v1/balance
This call returns the current channel balance that LND is reporting.

#### /api/v1/node_info
This call returns the node name, pubkey, and version that LND is reporting.

#### /api/v1/settings
This call returns the current settings that the user has set.

#### /api/v1/index
This call returns the most recent invoice index number that Helipad has reconciled with LND.

#### /api/v1/boosts
This call returns `count` boosts starting at `index`.  If the `old` parameter is present, the boosts returned start from `index` and
descend by `count`, showing older boosts.  Otherwise, they start at `index` and ascend by `count`, showing newer boosts.

#### /api/v1/streams
This call returns `count` streams starting at `index`.  If the `old` parameter is present, the streams returned start from `index` and
descend by `count`, showing older streams.  Otherwise, they start at `index` and ascend by `count`, showing newer streams.

#### /api/v1/sent_index
This call returns the most recent payment index number that Helipad has reconciled with LND.

#### /api/v1/sent
This call returns `count` sent boosts starting at `index`.  If the `old` parameter is present, the sent boosts returned start from `index` and
descend by `count`, showing older sent boosts.  Otherwise, they start at `index` and ascend by `count`, showing newer sent boosts.

<br><br>
## Webhooks
Webhooks send an HTTP POST to a user defined URL whenever a new boost, stream, or sent boost is processed by Helipad. The body of the POST will contain the following JSON format:
```json
{
  "direction": "incoming",
  "index": 1234,
  "time": 1714548166,
  "value_msat": 100000,
  "value_msat_total": 1000000,
  "action": 2,
  "sender": "Mark Pugner",
  "app": "CurioCaster",
  "message": "My boost message",
  "podcast": "Podcast name",
  "episode": "Episode name",
  "tlv": "{\n  \"podcast\": \"Podcast name\",\n  \"feedId\": 1234567,\n  \"episode\": \"Episode name\",\n  \"action\": \"boost\",\n  \"app_name\": \"CurioCaster\",\n  \"url\": \"https://podcast/example.xml\",\n  \"value_msat_total\": 1000000,\n  \"message\": \"My boost message\",\n  \"sender_name\": \"Mark Pugner\",\n  \"reply_address\": \"03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a\",\n  \"remote_feed_guid\": \"b8b6971e-403e-568f-a4e6-7aa2b45e50d4\",\n  \"remote_item_guid\": \"72a3b402-8491-4cd9-823e-a621fd81b86f\",\n  \"value_msat\": 100000,\n  \"name\": \"Podcastindex.org\"\n}\n",
  "remote_podcast": "Some artist",
  "remote_episode": "Some song",
  "reply_sent": false,
  "payment_info": null
}
```

The fields are as follows:

* `direction`: Payment direction ("incoming" or "outgoing")
* `index`: LND index of the invoice or payment
* `time`: Unix Timestamp of the item
* `value_msat`: Actual amount received/sent by our node
* `value_msat_total`: Total amount of the item
* `action`: Item type (1 = stream, 2 = boost, 3 = unknown, 4 = automated boost, 0 = error)
* `sender`: Sender's name
* `app`: Application used to send the item
* `message`: Message sent by the sender
* `podcast`: Name of the podcast
* `episode`: Name of the episode
* `tlv`: Raw copy of the TLV
* `remote_podcast`: Name of the remote podcast (during a Value Time Split)
* `remote_episode`: Name of the remote episode (during a Value Time Split)
* `reply_sent`: Flag that indicates if this item has been sent a reply boost
* `memo`: Memo field from Lightning invoice
* `payment_info`: Additional payment info for sent boosts:
  ```json
    "payment_info": {
      "payment_hash": "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX",
      "pubkey": "03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a",
      "custom_key": 696969,
      "custom_value": "XXXXXXXXXXXXXXXXXXXX",
      "fee_msat": 333,
      "reply_to_idx": null
    }
  ```
  * `payment_hash`: Payment hash from LND
  * `pubkey`: Recipient's node pubkey
  * `custom_key`: Recipient's wallet key
  * `custom_value`: Recipient's wallet ID
  * `fee_msat`: Fee paid to send boost
  * `reply_to_idx`: Index of item that was replied to


<br><br>
## CSV export
There is an endpoint called `/csv` that will export boosts as a CSV list to make organizing easier.  The parameters behave just like the
`boosts` and `streams` endpoints, but also accept an `end` parameter which limits how far back in time the CSV export list goes.  An example
call would look like this:

```http
GET http://localhost:2112/csv?index=38097&count=13049&old=true&end=25048
```

This will give back a csv list of 13,049 boosts starting at index 38097 and descending back in time - but will not go past index number
25048.


<br><br>
## Development

### Quick Start (Debian/Linux Mint/Ubuntu/Pop!_OS) 

In order to run Helipad locally you need to install the Rust Compiler `rustc`, the Rust package manager `cargo`, and the needed shared
libraries `libssl-dev`/`libsqlite3-dev`. Clone the Github repo with `git clone ...` and enter the `helipad` directory. Note, all commands
going forward will need to be ran from this directory. `cargo run` will compile and run helipad. If Helipad fails to start you may need to
edit `helipad.conf` or set/unset Environment Variables.

```sh
sudo apt install rustc cargo libssl-dev libsqlite3-dev
git clone https://github.com/Podcastindex-org/helipad.git
cd helipad
# Edit helipad.conf as needed
cargo run
# Open http://127.0.0.1:2112 in your browser
```

There is an [example build script](testbuildrun.sh) for quick compile/run when iterating on new features.  The script sets a proper environment, does a debug
compile and runs the executable.