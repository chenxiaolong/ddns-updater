### Version 0.1.12

* Update all dependencies ([PR #18])
* Use musl for prebuilt x86_64 Linux binary ([PR #18])

### Version 0.1.11

* Update all dependencies ([PR #12], [PR #13], [PR #14], [PR #15])
* Switch project layout to match my other projects ([PR #16])
    * Sign all prebuilt binaries
    * Use fat executables for macOS (x86_64 + aarch64)
    * Add Android aarch64 builds
    * Put changelog in `CHANGELOG.md` instead of Github Release metadata

### Version 0.1.10

Switched to the `tracing` library for logging and updated all dependencies ([PR #11]).

### Version 0.1.9

Updated all dependencies to their latest versions, updated to the latest Rust edition, and reformatted code with `cargo fmt` ([PR #8], [PR #9], [PR #10]). No changes to ddns-updater itself.

### Version 0.1.8

Updated all dependencies to their latest versions ([PR #7]). No changes to ddns-updater itself.

### Version 0.1.7

Dependency updates only ([PR #6]):

* clap 4.0.26
* env_logger 0.9.3
* gethostname 0.4.0
* netif 0.1.6
* serde 1.0.147
* serde_with 2.1.0
* thiserror 1.0.37

### Version 0.1.6

Dependency updates only ([PR #5]):

* clap 3.2.20
* log 0.4.17
* serde 1.0.144
* serde_with 2.0.0
* thiserror 1.0.33
* toml 0.5.9
* trust-dns-client 0.22.0

### Version 0.1.5

Dependency updates only ([PR #4]):

* clap 3.1.7
* gethostname 0.2.3
* log 0.4.16
* netif 0.1.3
* trust-dns-client 0.21.2

### Version 0.1.4

Dependency updates only ([PR #3]):

* trust-dns-client 0.21.0 (alpha.5 -> stable)
* Minor version updates for all remaining dependencies

### Version 0.1.3

Dependency updates only:

* trust-dns-client 0.21.0-alpha.5
* serde_with 1.12.0

### Version 0.1.2

Updated all dependencies to their latest versions. No changes to ddns-updater itself ([PR #2]).

### Version 0.1.1

This version switches to the [netif](https://github.com/bnoordhuis/netif) library for querying network interfaces ([PR #1]). The previous library had two memory leaks (missing frees for `malloc` and `getifaddrs`) and also dereferences a NULL pointer (`(struct ifaddrs).ifa_addr`) when an interface is layer 3 only (like Wireguard interfaces on Linux).

### Version 0.1.0

Initial release

[PR #1]: https://github.com/chenxiaolong/ddns-updater/pull/1
[PR #2]: https://github.com/chenxiaolong/ddns-updater/pull/2
[PR #3]: https://github.com/chenxiaolong/ddns-updater/pull/3
[PR #4]: https://github.com/chenxiaolong/ddns-updater/pull/4
[PR #5]: https://github.com/chenxiaolong/ddns-updater/pull/5
[PR #6]: https://github.com/chenxiaolong/ddns-updater/pull/6
[PR #7]: https://github.com/chenxiaolong/ddns-updater/pull/7
[PR #8]: https://github.com/chenxiaolong/ddns-updater/pull/8
[PR #9]: https://github.com/chenxiaolong/ddns-updater/pull/9
[PR #10]: https://github.com/chenxiaolong/ddns-updater/pull/10
[PR #11]: https://github.com/chenxiaolong/ddns-updater/pull/11
[PR #12]: https://github.com/chenxiaolong/ddns-updater/pull/12
[PR #13]: https://github.com/chenxiaolong/ddns-updater/pull/13
[PR #14]: https://github.com/chenxiaolong/ddns-updater/pull/14
[PR #15]: https://github.com/chenxiaolong/ddns-updater/pull/15
[PR #16]: https://github.com/chenxiaolong/ddns-updater/pull/16
[PR #18]: https://github.com/chenxiaolong/ddns-updater/pull/18
