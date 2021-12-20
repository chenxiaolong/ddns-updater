# ddns-updater

ddns-updater is a simple RFC 2136 dynamic DNS updater.

Features:
* Supports TSIG with hmac-sha256/sha384/sha512
* Automatically detects the correct interface based on the TCP source IP of a connection to the DNS server
* Suuports DDNS updates over both TCP and UDP

## Usage

First, make a copy of [`config.sample.toml`](config.sample.toml) and update the configuration to matched the desired DNS configuration. Then, run `ddns-updater` with:

```bash
ddns-updater --config config.toml
```

## Building from source

To build from source, first make sure that the Rust toolchain is installed. It can be installed from https://rustup.rs/ or the OS's package manager.

Build using the following command:

```
cargo build --release
```

The resulting executable will be in `target/release/ddns-updater` or `target\release\ddns-updater.exe`.

## Troubleshooting

If an issue occurs, the best way to troubleshoot is to set `log_level` to `debug` or `trace` in the config file. The `debug` level includes information like the detected IP addresses, while the `trace` level will also print out the raw DNS update request and response. Note that the `trace` output is not safe to paste online because it includes a dump of the TSIG key.

To enable trace logging for everything, including the underlying trust-dns library, set the `RUST_LOG` environment variable to `trace`.

## Limitations

* All IP addresses from the selected interface are used. It is not possible to filter for specific IPs.
* It is not possible to only update A records or only update AAAA records if the interface is dual-stack.
* Querying the "public" IP from online services will never be supported.
* Dynamic updates are made atomically by clearing all existing A/AAAA records and inserting the new records in the same request. However, some servers may not conform perfectly to RFC 2136 and may not implement the atomicity guarantees.

## License

ddns-updater is licensed under the GPLv3 license. For details, please see [`LICENSE`](./LICENSE).
