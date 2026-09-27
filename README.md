# lsm303-driver

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
use lsm303_driver::{LSM303, D, SA0};

// `i2c` is any `embedded_hal_async::i2c::I2c` implementation.
let mut sensor = LSM303::<_, D>::new(i2c, SA0::High);
sensor.enable_default().await?;

let (accel, mag) = sensor.read().await?;
// accel.x, accel.y, accel.z, mag.x, mag.y, mag.z (raw i16 counts)
```

Don't know which chip you have? Let the driver detect it:

```rust
use lsm303_driver::{AnyLsm303, LSM303, D};

let found = AnyLsm303::detect(&mut i2c, None, None).await?;

let mut sensor: LSM303<_, D> = match found.into_driver(i2c) {
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
