# ⚗ ogawa-rs
[![Actions Status](https://github.com/Traverse-Research/ogawa-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Traverse-Research/rust-template/actions)
[![Latest version](https://img.shields.io/crates/v/ogawa-rs.svg?logo=rust)](https://crates.io/crates/ogawa-rs)
[![Docs](https://docs.rs/ogawa-rs/badge.svg)](https://docs.rs/ogawa-rs/)
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![LICENSE](https://img.shields.io/badge/license-apache-blue.svg)](LICENSE-APACHE)
[![MSRV](https://img.shields.io/badge/rustc-1.74.0+-ab6000.svg)](https://blog.rust-lang.org/2023/11/16/Rust-1.74.0.html)
[![Contributor Covenant](https://img.shields.io/badge/contributor%20covenant-v1.4%20adopted-ff69b4.svg)](./CODE_OF_CONDUCT.md)

[![Banner](banner.png)](https://traverseresearch.nl)

This is a work in progress crate for loading Ogawa Alembic Cache files in Rust.
It currently only supports basic parsing of files and partially reading curves schemas.


```toml
[dependencies]
ogawa-rs = "0.4.0"
```

### License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../master/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](../master/LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

### Examples
Examples are located in the `/examples` directory and can be run by:

```bash
cargo run --example curves-test-vis /path/to/file.abc
```
```bash
cargo run --example print-tree /path/to/file.abc
```
```bash
cargo run --example schema-parsing /path/to/file.abc
```