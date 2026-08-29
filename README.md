# Uptime.rs

Minimal uptime monitoring service, written in Rust.

I felt all the existing uptime monitoring solutions were too bloated and not
easy to configure. This service has the following features:

* No JS
* Configurable with one YAML file (no clickops)

## Building

Notifications are delivered by
[shoutrrr](https://github.com/containrrr/shoutrrr), a Go library linked in
as a static archive, so a Go toolchain is needed alongside Rust. `build.rs`
compiles the archive automatically:

```sh
cargo build --release
```

To build against a prebuilt `libshoutrrr.a` instead — as the container and
CI builds do, so their Rust image needs no Go toolchain — point
`SHOUTRRR_LIB_DIR` at the directory containing it:

```sh
SHOUTRRR_LIB_DIR=/path/to/lib cargo build --release
```
