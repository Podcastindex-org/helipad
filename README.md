# Helipad
This package will poll a Lightning LND node for invoices related to Podcasting 2.0 and display them in a web interface.  It's
intended for use as a way to see incoming Boost-a-grams and other data from podcast listeners in a browser.  It will be made
available as an Umbrel app soon.

Helipad runs as a single process web server with the LND poller running as a separate thread.  Invoices are checked for every
9 seconds, parsed and stored locally in a Sqlite database.  The main webserver thread then serves them to clients over HTTP(S).

After compiling, you start the binary like this:

```./helipad 8080```

You must pass the port number you want it to listen on, on the command line as the only argument.

The FQDN of your LND node must be present in an environment variable called $LND_URL, like this:

```export LND_URL="mynode.example.com:10009"```

It will expect to find your `tls.cert` and `admin.macaroon` files in the directory you run it from.  In the future, these
paths will change to become specifiable as environment variables also.
