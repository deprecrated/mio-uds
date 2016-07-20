# mio-uds

[![Build Status](https://travis-ci.org/alexcrichton/mio-uds.svg?branch=master)](https://travis-ci.org/alexcrichton/mio-uds)

[Documentation](http://alexcrichton.com/mio-uds/mio-uds)

A library for integrating Unix Domain Sockets with [mio]. Based on the standard
library's [support for Unix sockets][std], except all of the abstractions and
types are nonblocking to conform with the expectations of mio.

[mio]: https://github.com/carllerche/mio
[std]: https://doc.rust-lang.org/std/os/unix/net/

```toml
# Cargo.toml
[dependencies]
mio-uds = { git = "https://github.com/alexcrichton/mio-uds" }
mio = { git = "https://github.com/carllerche/mio" }
```

> **Note**: This library depends on the unreleased 0.6.0 version of mio, so
> you'll need to also depend on the `master` branch of mio for now to use it.

## Usage

The three exported types at the top level, `UnixStream`, `UnixListener`, and
`UnixDatagram`, are thin wrappers around the libstd counterparts. They can be
used in similar fashions to mio's TCP and UDP types in terms of registration and
API.

# License

`mio-uds` is primarily distributed under the terms of both the MIT license and
the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.

