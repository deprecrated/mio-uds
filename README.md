# mio-uds

Deprecated library for integrating Unix Domain Sockets with [mio].

Use the `uds` feature on mio instead of this crate:

[mio]: https://github.com/carllerche/mio

```toml
# Cargo.toml
[dependencies]
mio = { version = "0.7", features = ["uds"] }
```

## Usage

The three exported types at the top level, `UnixStream`, `UnixListener`, and
`UnixDatagram`, are reexports from mio.

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
