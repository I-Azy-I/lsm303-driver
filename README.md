# lsm303-driver

[![CI](https://github.com/I-Azy-I/lsm303-driver/actions/workflows/ci.yml/badge.svg)](https://github.com/I-Azy-I/lsm303-driver/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/lsm303-driver.svg)](https://crates.io/crates/lsm303-driver)
[![docs.rs](https://docs.rs/lsm303-driver/badge.svg)](https://docs.rs/lsm303-driver)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![no_std](https://img.shields.io/badge/no__std-yes-green.svg)](https://docs.rust-embedded.org/book/intro/no-std.html)

A `no_std` Rust driver for the ST LSM303 accelerometer + magnetometer family
(also sold as the **GY-511** breakout board):

- LSM303DLH
- LSM303DLM
- LSM303DLHC
- LSM303D

Built on `embedded-hal` 1.0. It is async by default; enable the `sync` feature
for the blocking API.

## Usage

```rust
use lsm303_driver::{Lsm303, D, SA0};

// `i2c` is any `embedded_hal_async::i2c::I2c` implementation.
let mut sensor = Lsm303::<_, D>::new(i2c, SA0::High);
sensor.enable_default().await?;

let (accel, mag) = sensor.read().await?;
// accel.x, accel.y, accel.z, mag.x, mag.y, mag.z (raw i16 counts)
```

Don't know which chip you have? Let the driver detect it:

```rust
use lsm303_driver::{AnyLsm303, Lsm303, D};

let found = AnyLsm303::detect(&mut i2c, None, None).await?;

let mut sensor: Lsm303<_, D> = match found.into_driver(i2c) {
    Ok(sensor) => sensor,
    Err(other) => panic!("expected an LSM303D, found {:?}", other),
};
sensor.enable_default().await?;
```

For the blocking version:

```toml
lsm303-driver = { version = "0.1", default-features = false, features = ["sync"] }
```

Then drop the `.await`s.

## Status

Tested only on the **LSM303D**. The other variants are implemented from their
datasheets but not yet tested on hardware.

## Credits

Based on Pololu's [LSM303 Arduino library](https://github.com/pololu/lsm303-arduino):
device detection, register map and default configuration are ported from it.
Pololu's copyright and MIT license notice is kept in [LICENSE-POLOLU](LICENSE-POLOLU).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
