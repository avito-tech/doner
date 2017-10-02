# Doner
Doner is a service that downloads files and serves them to local clients over HTTP
It was written at one of the internal hackatons as an answer to some internal problems

The project is written in Rust, using Tokio framework.

## What problems we had
There are many programmer tasks in different projects were we need to download many files from different locations

Every time one needs to download a file from Internet, many problems need to be solved:
- supporting many different protocols: HTTP(S), FTP, SCP, S3, etc.
- remote server can be very slow or even stalled eventually, and we can do nothing with it
- all downloads go synchronously, big files, slow loading of small ones in that case, and every programmer id forced to deal with async model

## What we suggested
We add the AMQP server, where user can add a download job to some queue. Multiple doners, working in a cluster pop jobs from the queue saving them locally.
After that they to a correct download in asynchronous mode putting the results to their own storages. These storages are available via HTTP.
User must provide a callback location where the link to finished download(or an error message) is to be sent.

Here's how download job looks like in JSON

The job contains the URL to download, and instructions about where and how the download result should be delivered back
```json
{
  "url": "http://speedtest.tele2.net/1MB.zip",
  "reply_to": {
    "endpoint": "url",
    "target": "http://127.0.0.1/downloader/callback"
  }
}
```

Currently we support following delivery mechanisms (both are specified as `reply_to.target`:
- `url` - an URL to callback and pass the link to downloaded file
- `queue` - an AMQP-queue name to place link to

For better resistance to doner restarts and failures, we add another per-doner AMQP queue to store pending jobs. Before doner starts downloading, 
it puts the job to local queue, giving more chances the job will not be lost.

## Configuration
All configuration parameters are explained in config.example.yaml

## Building
You only need Rust toolchain to build everything. It can be installed using your pakcage manager or [Rustup](http://rustup.rs)

After having one, just run

`cargo build --release`

and you are ready to go

#### If you have problems building on MacOS, try this:

```
export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
cargo clean
cargo test
```

## Running

```
cargo run --release
```

## Testing
For testing you will also need:
- make
- openssl
- docker

After getting them run:
- `make prepare` - to generate certificate and testing config
- `make start` - to start a docker container with RabbitmQ
- `cargo run` - to start a debug version of doner server
- `make pub`/`make pubq`/`make pubs` to do publish some test jobs to AMQP which should be processed by doner. Commands differ 
- `make err` - to generate incorrect job and test some basic error processing
- `make stop` - to stop the container


## Contributing
To help project you can:
* solve issues, making PRs
* find unwraps and process the returned errors correctly without panicking
* test this at your site, creating new issues if you manage to find any

