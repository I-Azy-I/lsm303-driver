#![no_std]

// `sync` and `async` select which `embedded-hal` I2c trait the driver is built
// against, so exactly one must be active. Cargo features are additive, so guard
// against a dependency turning on the other one behind your back.
#[cfg(all(feature = "sync", feature = "async"))]
compile_error!(
    "features `sync` and `async` are mutually exclusive - enable exactly one \
     (e.g. `default-features = false, features = [\"sync\"]`)"
);

#[cfg(not(any(feature = "sync", feature = "async")))]
compile_error!("enable exactly one of the `sync` or `async` features");

mod lsm303_registers;

use lsm303_registers::*;

use crate::lsm303_registers::reg_addr::{TEMP_OUT_H, TEMP_OUT_L};
use core::marker::PhantomData;
#[cfg(feature = "sync")]
use embedded_hal::i2c::{Error as _, ErrorKind, I2c};
#[cfg(feature = "async")]
use embedded_hal_async::i2c::{Error as _, ErrorKind, I2c};

// Full-scale range enums.
//
// Each `bits()` returns the register field value already shifted into position,
// and `MASK` is that field's bit mask, so a setter does a read-modify-write of
// exactly those bits.
//
// The bit encodings below are verified against the ST datasheets (LSM303DLH,
// LSM303DLHC, LSM303D). Note DLH/DLM encode 8 g as `FS = 0b11` while the DLHC
// uses `0b10`, which is why the accelerometer scales are separate enums.

/// Magnetometer full-scale range for the DLH/DLM/DLHC (`GN[2:0]` in `CRB_REG_M`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlxMagScale {
    G1_3,
    G1_9,
    G2_5,
    G4_0,
    G4_7,
    G5_6,
    G8_1,
}

impl DlxMagScale {
    /// `GN[2:0]` field mask in `CRB_REG_M` (bits 7:5).
    const MASK: u8 = 0b1110_0000;

    fn bits(self) -> u8 {
        match self {
            DlxMagScale::G1_3 => 0b001 << 5,
            DlxMagScale::G1_9 => 0b010 << 5,
            DlxMagScale::G2_5 => 0b011 << 5,
            DlxMagScale::G4_0 => 0b100 << 5,
            DlxMagScale::G4_7 => 0b101 << 5,
            DlxMagScale::G5_6 => 0b110 << 5,
            DlxMagScale::G8_1 => 0b111 << 5,
        }
    }
}

/// Magnetometer full-scale range for the LSM303D (`MFS[1:0]` in `CTRL6`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DMagScale {
    G2,
    G4,
    G8,
    G12,
}

impl DMagScale {
    /// `MFS[1:0]` field mask in `CTRL6` (bits 6:5).
    const MASK: u8 = 0b0110_0000;

    fn bits(self) -> u8 {
        match self {
            DMagScale::G2 => 0b00 << 5,
            DMagScale::G4 => 0b01 << 5,
            DMagScale::G8 => 0b10 << 5,
            DMagScale::G12 => 0b11 << 5,
        }
    }
}

/// Accelerometer full-scale range for the DLH and DLM (`FS[1:0]` in `CTRL_REG4_A`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlhDlmAccScale {
    G2,
    G4,
    G8,
}

impl DlhDlmAccScale {
    /// `FS[1:0]` field mask in `CTRL_REG4_A` (bits 5:4).
    const MASK: u8 = 0b0011_0000;

    fn bits(self) -> u8 {
        // NB: DLH/DLM encode 8 g as FS = 0b11 (0b10 is unused).
        match self {
            DlhDlmAccScale::G2 => 0b00 << 4,
            DlhDlmAccScale::G4 => 0b01 << 4,
            DlhDlmAccScale::G8 => 0b11 << 4,
        }
    }

    /// Decode the current range from a raw `CTRL_REG4_A` value.
    fn from_reg(reg: u8) -> Self {
        match reg & Self::MASK {
            0x10 => DlhDlmAccScale::G4,
            0x30 => DlhDlmAccScale::G8,
            _ => DlhDlmAccScale::G2,
        }
    }
}

/// Accelerometer full-scale range for the DLHC (`FS[1:0]` in `CTRL_REG4_A`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlhcAccScale {
    G2,
    G4,
    G8,
    G16,
}

impl DlhcAccScale {
    /// `FS[1:0]` field mask in `CTRL_REG4_A` (bits 5:4).
    const MASK: u8 = 0b0011_0000;

    fn bits(self) -> u8 {
        // NB: the DLHC encodes 8 g as FS = 0b10 (unlike DLH/DLM's 0b11).
        match self {
            DlhcAccScale::G2 => 0b00 << 4,
            DlhcAccScale::G4 => 0b01 << 4,
            DlhcAccScale::G8 => 0b10 << 4,
            DlhcAccScale::G16 => 0b11 << 4,
        }
    }

    /// Decode the current range from a raw `CTRL_REG4_A` value.
    fn from_reg(reg: u8) -> Self {
        match reg & Self::MASK {
            0x10 => DlhcAccScale::G4,
            0x20 => DlhcAccScale::G8,
            0x30 => DlhcAccScale::G16,
            _ => DlhcAccScale::G2,
        }
    }
}

/// Accelerometer full-scale range for the LSM303D (`AFS[2:0]` in `CTRL2`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DAccScale {
    G2,
    G4,
    G6,
    G8,
    G16,
}

impl DAccScale {
    /// `AFS[2:0]` field mask in `CTRL2` (bits 5:3).
    const MASK: u8 = 0b0011_1000;

    fn bits(self) -> u8 {
        match self {
            DAccScale::G2 => 0b000 << 3,
            DAccScale::G4 => 0b001 << 3,
            DAccScale::G6 => 0b010 << 3,
            DAccScale::G8 => 0b011 << 3,
            DAccScale::G16 => 0b100 << 3,
        }
    }

    /// Decode the current range from a raw `CTRL2` value.
    fn from_reg(reg: u8) -> Self {
        match reg & Self::MASK {
            0x08 => DAccScale::G4,
            0x10 => DAccScale::G6,
            0x18 => DAccScale::G8,
            0x20 => DAccScale::G16,
            _ => DAccScale::G2,
        }
    }
}

// Output-data-rate enums (verified against the ST datasheets).

/// Accelerometer output data rate for the DLH and DLM (`CTRL_REG1_A`).
///
/// These chips select the rate via the `DR` field but only produce data in
/// normal power mode, so setting a rate also forces `PM = normal`; `PowerDown`
/// sets `PM = power-down`. Encoded across `PM[2:0]` (bits 7:5) and `DR[1:0]`
/// (bits 4:3) together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlhDlmAccOdr {
    PowerDown,
    Hz50,
    Hz100,
    Hz400,
    Hz1000,
}

impl DlhDlmAccOdr {
    /// `PM[2:0]` + `DR[1:0]` field mask in `CTRL_REG1_A` (bits 7:3).
    const MASK: u8 = 0b1111_1000;

    fn bits(self) -> u8 {
        // PM = 001 (normal) with the DR field, except PowerDown = PM 000.
        match self {
            DlhDlmAccOdr::PowerDown => 0b000_00 << 3,
            DlhDlmAccOdr::Hz50 => 0b001_00 << 3,
            DlhDlmAccOdr::Hz100 => 0b001_01 << 3,
            DlhDlmAccOdr::Hz400 => 0b001_10 << 3,
            DlhDlmAccOdr::Hz1000 => 0b001_11 << 3,
        }
    }
}

/// Accelerometer output data rate for the DLHC (`ODR[3:0]` in `CTRL_REG1_A`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlhcAccOdr {
    PowerDown,
    Hz1,
    Hz10,
    Hz25,
    Hz50,
    Hz100,
    Hz200,
    Hz400,
}

impl DlhcAccOdr {
    /// `ODR[3:0]` field mask in `CTRL_REG1_A` (bits 7:4).
    const MASK: u8 = 0b1111_0000;

    fn bits(self) -> u8 {
        match self {
            DlhcAccOdr::PowerDown => 0b0000 << 4,
            DlhcAccOdr::Hz1 => 0b0001 << 4,
            DlhcAccOdr::Hz10 => 0b0010 << 4,
            DlhcAccOdr::Hz25 => 0b0011 << 4,
            DlhcAccOdr::Hz50 => 0b0100 << 4,
            DlhcAccOdr::Hz100 => 0b0101 << 4,
            DlhcAccOdr::Hz200 => 0b0110 << 4,
            DlhcAccOdr::Hz400 => 0b0111 << 4,
        }
    }
}

/// Accelerometer output data rate for the LSM303D (`AODR[3:0]` in `CTRL1`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DAccOdr {
    PowerDown,
    Hz3_125,
    Hz6_25,
    Hz12_5,
    Hz25,
    Hz50,
    Hz100,
    Hz200,
    Hz400,
    Hz800,
    Hz1600,
}

impl DAccOdr {
    /// `AODR[3:0]` field mask in `CTRL1` (bits 7:4).
    const MASK: u8 = 0b1111_0000;

    fn bits(self) -> u8 {
        match self {
            DAccOdr::PowerDown => 0b0000 << 4,
            DAccOdr::Hz3_125 => 0b0001 << 4,
            DAccOdr::Hz6_25 => 0b0010 << 4,
            DAccOdr::Hz12_5 => 0b0011 << 4,
            DAccOdr::Hz25 => 0b0100 << 4,
            DAccOdr::Hz50 => 0b0101 << 4,
            DAccOdr::Hz100 => 0b0110 << 4,
            DAccOdr::Hz200 => 0b0111 << 4,
            DAccOdr::Hz400 => 0b1000 << 4,
            DAccOdr::Hz800 => 0b1001 << 4,
            DAccOdr::Hz1600 => 0b1010 << 4,
        }
    }
}

/// Magnetometer output data rate for the DLH and DLM (`DO[2:0]` in `CRA_REG_M`).
///
/// These two cap at 75 Hz (`DO = 111` is "not used" per the datasheets). The
/// DLHC additionally supports 220 Hz — see [`DlhcMagOdr`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlhDlmMagOdr {
    Hz0_75,
    Hz1_5,
    Hz3,
    Hz7_5,
    Hz15,
    Hz30,
    Hz75,
}

impl DlhDlmMagOdr {
    /// `DO[2:0]` field mask in `CRA_REG_M` (bits 4:2).
    const MASK: u8 = 0b0001_1100;

    fn bits(self) -> u8 {
        match self {
            DlhDlmMagOdr::Hz0_75 => 0b000 << 2,
            DlhDlmMagOdr::Hz1_5 => 0b001 << 2,
            DlhDlmMagOdr::Hz3 => 0b010 << 2,
            DlhDlmMagOdr::Hz7_5 => 0b011 << 2,
            DlhDlmMagOdr::Hz15 => 0b100 << 2,
            DlhDlmMagOdr::Hz30 => 0b101 << 2,
            DlhDlmMagOdr::Hz75 => 0b110 << 2,
        }
    }
}

/// Magnetometer output data rate for the DLHC (`DO[2:0]` in `CRA_REG_M`).
///
/// Same table as the DLH/DLM plus 220 Hz at `DO = 111` (verified against the
/// LSM303DLHC datasheet, Table 72).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DlhcMagOdr {
    Hz0_75,
    Hz1_5,
    Hz3,
    Hz7_5,
    Hz15,
    Hz30,
    Hz75,
    Hz220,
}

impl DlhcMagOdr {
    /// `DO[2:0]` field mask in `CRA_REG_M` (bits 4:2).
    const MASK: u8 = 0b0001_1100;

    fn bits(self) -> u8 {
        match self {
            DlhcMagOdr::Hz0_75 => 0b000 << 2,
            DlhcMagOdr::Hz1_5 => 0b001 << 2,
            DlhcMagOdr::Hz3 => 0b010 << 2,
            DlhcMagOdr::Hz7_5 => 0b011 << 2,
            DlhcMagOdr::Hz15 => 0b100 << 2,
            DlhcMagOdr::Hz30 => 0b101 << 2,
            DlhcMagOdr::Hz75 => 0b110 << 2,
            DlhcMagOdr::Hz220 => 0b111 << 2,
        }
    }
}

/// Magnetometer output data rate for the LSM303D (`M_ODR[2:0]` in `CTRL5`).
///
/// 100 Hz is only valid when the accelerometer ODR is above 50 Hz (or in
/// power-down); see the datasheet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DMagOdr {
    Hz3_125,
    Hz6_25,
    Hz12_5,
    Hz25,
    Hz50,
    Hz100,
}

impl DMagOdr {
    /// `M_ODR[2:0]` field mask in `CTRL5` (bits 4:2).
    const MASK: u8 = 0b0001_1100;

    fn bits(self) -> u8 {
        match self {
            DMagOdr::Hz3_125 => 0b000 << 2,
            DMagOdr::Hz6_25 => 0b001 << 2,
            DMagOdr::Hz12_5 => 0b010 << 2,
            DMagOdr::Hz25 => 0b011 << 2,
            DMagOdr::Hz50 => 0b100 << 2,
            DMagOdr::Hz100 => 0b101 << 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Device {
    Dlh,
    Dlm,
    Dlhc,
    D,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SA0 {
    Low,
    High,
}

/// Errors returned by the driver.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    I2c(E),
    DeviceNotFound,
}

/// A 3-axis reading (accelerometer or magnetometer), in raw sensor counts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Vector {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

/// I²C addresses and WHO_AM_I IDs used only for device detection.
mod detect {
    pub const D_SA0_HIGH: u8 = 0x1D; // 0b0011101
    pub const D_SA0_LOW: u8 = 0x1E; // 0b0011110
    pub const DLX_ACC_SA0_HIGH: u8 = 0x19; // 0b0011001
    pub const DLX_ACC_SA0_LOW: u8 = 0x18; // 0b0011000
    pub const DLX_MAG: u8 = 0x1E; // 0b0011110

    pub const WHO_ID_D: u8 = 0x49;
    pub const WHO_ID_DLM: u8 = 0x3C;
}
pub struct Dlh;
pub struct Dlm;
pub struct Dlhc;
pub struct D;

/// Marker for the DLH/DLM/DLHC family, which share the magnetometer full-scale
/// (`GN`) table and register layout. Gates methods common to those three.
pub trait Dlx: Variant {}
impl Dlx for Dlh {}
impl Dlx for Dlm {}
impl Dlx for Dlhc {}

/// Per-variant hardware differences: I²C addresses, magnetometer output
/// layout, and the power-on default configuration. Implemented by the marker
/// types [`Dlh`], [`Dlm`], [`Dlhc`], and [`D`].
///
/// This replaces the Arduino library's runtime `translated_regs` table and
/// `_device` switches: each chip's quirks live in its trait impl and are
/// resolved at compile time.
// The returned futures are only awaited inside this crate / on a single-core
// embassy executor, so `Send` bounds are unnecessary. Same choice as
// `embedded-hal-async` makes for its own traits.
#[allow(async_fn_in_trait)]
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
pub trait Variant: Sized {
    /// The runtime [`Device`] this marker type corresponds to.
    ///
    /// Lets a runtime detection result be checked against the compile-time
    /// variant; see [`AnyLsm303::into_driver`].
    const DEVICE: Device;

    /// First register of a 6-byte magnetometer burst read, with the
    /// auto-increment bit already set where the chip requires it.
    const MAG_OUT_START: u8;

    /// Accelerometer I²C address for the given SA0 state.
    fn acc_address(sa0: SA0) -> u8;

    /// Magnetometer I²C address for the given SA0 state.
    fn mag_address(sa0: SA0) -> u8;

    /// Reassemble the 6 magnetometer bytes (in this chip's output order)
    /// into a signed X/Y/Z vector.
    fn parse_mag(buf: &[u8; 6]) -> Vector;

    /// Apply the power-on default configuration:
    /// enable both sensors at their datasheet-specified
    /// output data rates and full scales.
    async fn enable_default<I2C: I2c>(dev: &mut LSM303<I2C, Self>)
    -> Result<(), Error<I2C::Error>>;
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl Variant for D {
    const DEVICE: Device = Device::D;

    // D returns X_L, X_H, Y_L, Y_H, Z_L, Z_H, and needs the auto-increment bit.
    const MAG_OUT_START: u8 = 0x08 | 0x80; // D_OUT_X_L_M | auto-increment

    fn acc_address(sa0: SA0) -> u8 {
        match sa0 {
            SA0::High => detect::D_SA0_HIGH,
            SA0::Low => detect::D_SA0_LOW,
        }
    }

    // On the D the accelerometer and magnetometer share one address.
    fn mag_address(sa0: SA0) -> u8 {
        Self::acc_address(sa0)
    }

    fn parse_mag(buf: &[u8; 6]) -> Vector {
        Vector {
            x: i16::from_le_bytes([buf[0], buf[1]]),
            y: i16::from_le_bytes([buf[2], buf[3]]),
            z: i16::from_le_bytes([buf[4], buf[5]]),
        }
    }

    async fn enable_default<I2C: I2c>(
        dev: &mut LSM303<I2C, Self>,
    ) -> Result<(), Error<I2C::Error>> {
        // acc_address == mag_address on the D, so either accessor works.
        dev.write_mag_reg(reg_addr::CTRL2, 0x00).await?; // accel +/-2 g
        dev.write_mag_reg(reg_addr::CTRL1, 0x57).await?; // accel 50 Hz, all axes on
        dev.write_mag_reg(reg_addr::CTRL5, 0x64).await?; // mag high-res, 6.25 Hz
        dev.write_mag_reg(reg_addr::CTRL6, 0x20).await?; // mag +/-4 gauss
        dev.write_mag_reg(reg_addr::CTRL7, 0x00).await?; // mag continuous-conversion
        Ok(())
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl Variant for Dlhc {
    const DEVICE: Device = Device::Dlhc;

    // DLHC returns X_H, X_L, Z_H, Z_L, Y_H, Y_L; its mag auto-increments on its own.
    const MAG_OUT_START: u8 = 0x03; // DLHC_OUT_X_H_M

    fn acc_address(_sa0: SA0) -> u8 {
        detect::DLX_ACC_SA0_HIGH // fixed; the DLHC has no configurable SA0
    }

    fn mag_address(_sa0: SA0) -> u8 {
        detect::DLX_MAG
    }

    fn parse_mag(buf: &[u8; 6]) -> Vector {
        Vector {
            x: i16::from_le_bytes([buf[1], buf[0]]),
            z: i16::from_le_bytes([buf[3], buf[2]]),
            y: i16::from_le_bytes([buf[5], buf[4]]),
        }
    }

    async fn enable_default<I2C: I2c>(
        dev: &mut LSM303<I2C, Self>,
    ) -> Result<(), Error<I2C::Error>> {
        dev.write_acc_reg(reg_addr::CTRL_REG4_A, 0x08).await?; // +/-2 g, high-res
        dev.write_acc_reg(reg_addr::CTRL_REG1_A, 0x47).await?; // 50 Hz, all axes on
        dev.write_mag_reg(reg_addr::CRA_REG_M, 0x0C).await?; // 7.5 Hz
        dev.write_mag_reg(reg_addr::CRB_REG_M, 0x20).await?; // +/-1.3 gauss
        dev.write_mag_reg(reg_addr::MR_REG_M, 0x00).await?; // continuous-conversion
        Ok(())
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl Variant for Dlm {
    const DEVICE: Device = Device::Dlm;

    // DLM returns X_H, X_L, Z_H, Z_L, Y_H, Y_L (same order as DLHC).
    const MAG_OUT_START: u8 = 0x03; // DLM_OUT_X_H_M

    fn acc_address(sa0: SA0) -> u8 {
        match sa0 {
            SA0::High => detect::DLX_ACC_SA0_HIGH,
            SA0::Low => detect::DLX_ACC_SA0_LOW,
        }
    }

    fn mag_address(_sa0: SA0) -> u8 {
        detect::DLX_MAG
    }

    fn parse_mag(buf: &[u8; 6]) -> Vector {
        Vector {
            x: i16::from_le_bytes([buf[1], buf[0]]),
            z: i16::from_le_bytes([buf[3], buf[2]]),
            y: i16::from_le_bytes([buf[5], buf[4]]),
        }
    }

    async fn enable_default<I2C: I2c>(
        dev: &mut LSM303<I2C, Self>,
    ) -> Result<(), Error<I2C::Error>> {
        dev.write_acc_reg(reg_addr::CTRL_REG4_A, 0x00).await?; // +/-2 g
        dev.write_acc_reg(reg_addr::CTRL_REG1_A, 0x27).await?; // normal, 50 Hz, all axes
        dev.write_mag_reg(reg_addr::CRA_REG_M, 0x0C).await?;
        dev.write_mag_reg(reg_addr::CRB_REG_M, 0x20).await?;
        dev.write_mag_reg(reg_addr::MR_REG_M, 0x00).await?;
        Ok(())
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl Variant for Dlh {
    const DEVICE: Device = Device::Dlh;

    // DLH returns X_H, X_L, Y_H, Y_L, Z_H, Z_L (Y before Z, unlike DLM/DLHC).
    const MAG_OUT_START: u8 = 0x03; // DLH_OUT_X_H_M

    fn acc_address(sa0: SA0) -> u8 {
        match sa0 {
            SA0::High => detect::DLX_ACC_SA0_HIGH,
            SA0::Low => detect::DLX_ACC_SA0_LOW,
        }
    }

    fn mag_address(_sa0: SA0) -> u8 {
        detect::DLX_MAG
    }

    fn parse_mag(buf: &[u8; 6]) -> Vector {
        Vector {
            x: i16::from_le_bytes([buf[1], buf[0]]),
            y: i16::from_le_bytes([buf[3], buf[2]]),
            z: i16::from_le_bytes([buf[5], buf[4]]),
        }
    }

    async fn enable_default<I2C: I2c>(
        dev: &mut LSM303<I2C, Self>,
    ) -> Result<(), Error<I2C::Error>> {
        dev.write_acc_reg(reg_addr::CTRL_REG4_A, 0x00).await?;
        dev.write_acc_reg(reg_addr::CTRL_REG1_A, 0x27).await?;
        dev.write_mag_reg(reg_addr::CRA_REG_M, 0x0C).await?;
        dev.write_mag_reg(reg_addr::CRB_REG_M, 0x20).await?;
        dev.write_mag_reg(reg_addr::MR_REG_M, 0x00).await?;
        Ok(())
    }
}

pub struct AnyLsm303 {
    device: Device,
    sa0: SA0,
}

#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl AnyLsm303 {
    /// Probe the bus and return a typed driver for whatever's found.
    pub async fn detect<I2C: I2c>(
        i2c: &mut I2C,
        device: Option<Device>,
        sa0: Option<SA0>,
    ) -> Result<Self, Error<I2C::Error>> {
        Self::probe(i2c, device, sa0).await
    }

    /// Probe the bus to resolve the device type and SA0 state.
    ///
    /// Ported from Pololu's `LSM303::init` detection logic.
    async fn probe<I2C: I2c>(
        i2c: &mut I2C,
        device: Option<Device>,
        sa0: Option<SA0>,
    ) -> Result<Self, Error<I2C::Error>> {
        // Fully specified → trust the caller, don't probe.
        if let (Some(d), Some(s)) = (device, sa0) {
            return Ok(AnyLsm303 { device: d, sa0: s });
        }

        // LSM303D
        if device.is_none() || device == Some(Device::D) {
            if sa0 != Some(SA0::Low)
                && Self::test_reg(i2c, detect::D_SA0_HIGH, reg_addr::WHO_AM_I).await?
                    == Some(detect::WHO_ID_D)
            {
                return Ok(AnyLsm303 {
                    device: Device::D,
                    sa0: SA0::High,
                });
            }
            if sa0 != Some(SA0::High)
                && Self::test_reg(i2c, detect::D_SA0_LOW, reg_addr::WHO_AM_I).await?
                    == Some(detect::WHO_ID_D)
            {
                return Ok(AnyLsm303 {
                    device: Device::D,
                    sa0: SA0::Low,
                });
            }
        }

        // LSM303DLHC / DLM / DLH
        let is_dlx = matches!(
            device,
            None | Some(Device::Dlhc) | Some(Device::Dlm) | Some(Device::Dlh)
        );
        if is_dlx {
            if sa0 != Some(SA0::Low)
                && Self::test_reg(i2c, detect::DLX_ACC_SA0_HIGH, reg_addr::CTRL_REG1_A)
                    .await?
                    .is_some()
            {
                // Keep the caller's device if pinned; otherwise disambiguate by mag ID.
                // DLHC reports the DLM mag ID, so a match here means DLHC.
                let dev = match device {
                    Some(d) => d,
                    None if Self::mag_is_dlm(i2c).await? => Device::Dlhc,
                    None => Device::Dlh,
                };
                return Ok(AnyLsm303 {
                    device: dev,
                    sa0: SA0::High,
                });
            }
            if sa0 != Some(SA0::High)
                && Self::test_reg(i2c, detect::DLX_ACC_SA0_LOW, reg_addr::CTRL_REG1_A)
                    .await?
                    .is_some()
            {
                let dev = match device {
                    Some(d) => d,
                    None if Self::mag_is_dlm(i2c).await? => Device::Dlm,
                    None => Device::Dlh,
                };
                return Ok(AnyLsm303 {
                    device: dev,
                    sa0: SA0::Low,
                });
            }
        }

        Err(Error::DeviceNotFound)
    }

    /// True if the magnetometer reports the DLM WHO_AM_I ID.
    async fn mag_is_dlm<I2C: I2c>(i2c: &mut I2C) -> Result<bool, Error<I2C::Error>> {
        Ok(
            Self::test_reg(i2c, detect::DLX_MAG, reg_addr::WHO_AM_I_M).await?
                == Some(detect::WHO_ID_DLM),
        )
    }

    /// Read one register at `address`.
    ///
    /// `Ok(None)` means the device didn't ACK (i.e. nothing is at that
    /// address); real bus faults propagate as `Err`.
    async fn test_reg<I2C: I2c>(
        i2c: &mut I2C,
        address: u8,
        register: u8,
    ) -> Result<Option<u8>, Error<I2C::Error>> {
        let mut buf = [0u8];
        // i2c.write_read(0x1d, &[0x0f], &mut who).unwrap();
        // info!("WHO_AM_I = 0x{:02x} (expect 0x49 for LSM303D)", who[0]);

        match i2c.write_read(address, &[register], &mut buf).await {
            Ok(()) => Ok(Some(buf[0])),
            Err(e) => match e.kind() {
                ErrorKind::NoAcknowledge(_) => Ok(None),
                _ => Err(Error::I2c(e)),
            },
        }
    }
}

impl AnyLsm303 {
    /// The device type that was detected.
    pub fn device(&self) -> Device {
        self.device
    }

    /// The SA0 state that was detected.
    pub fn sa0(&self) -> SA0 {
        self.sa0
    }

    /// Turn a detection result into a typed driver, taking ownership of the bus.
    pub fn into_driver<I2C: I2c, V: Variant>(self, i2c: I2C) -> Result<LSM303<I2C, V>, Device> {
        if self.device != V::DEVICE {
            return Err(self.device);
        }
        Ok(LSM303::new_typed(i2c, self.sa0))
    }
}

#[allow(non_snake_case)]
pub struct LSM303<I2C, V> {
    _variant: PhantomData<V>,
    i2c: I2C,
    sa0: SA0,
}

impl<I2C: I2c, V> LSM303<I2C, V> {
    /// Wrap an already-identified device.
    ///
    /// Internal: used by [`AnyLsm303::detect`] and the per-variant
    /// constructors once the variant `V` is known.
    fn new_typed(i2c: I2C, sa0: SA0) -> Self {
        Self {
            _variant: PhantomData,
            i2c,
            sa0,
        }
    }

    /// Consume the driver and release the underlying I²C bus.
    pub fn release(self) -> I2C {
        self.i2c
    }
}

// Direct constructors that skip detection, use when you already know which
// chip is on the bus. Only the variant you name is monomorphized, so unused
// variants are stripped from the binary.
impl<I2C: I2c> LSM303<I2C, Dlh> {
    /// Create a driver for an LSM303DLH at the given SA0 address.
    pub fn new(i2c: I2C, sa0: SA0) -> Self {
        Self::new_typed(i2c, sa0)
    }
}

impl<I2C: I2c> LSM303<I2C, Dlm> {
    /// Create a driver for an LSM303DLM at the given SA0 address.
    pub fn new(i2c: I2C, sa0: SA0) -> Self {
        Self::new_typed(i2c, sa0)
    }
}

impl<I2C: I2c> LSM303<I2C, Dlhc> {
    /// Create a driver for an LSM303DLHC at the given SA0 address.
    pub fn new(i2c: I2C, sa0: SA0) -> Self {
        Self::new_typed(i2c, sa0)
    }
}

impl<I2C: I2c> LSM303<I2C, D> {
    /// Create a driver for an LSM303D at the given SA0 address.
    pub fn new(i2c: I2C, sa0: SA0) -> Self {
        Self::new_typed(i2c, sa0)
    }
}

// Operational methods, available once the variant `V` is known.
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c, V: Variant> LSM303<I2C, V> {
    /// Write `value` to `reg` at the given I²C `address`.
    async fn write_register(
        &mut self,
        address: u8,
        reg: u8,
        value: u8,
    ) -> Result<(), Error<I2C::Error>> {
        self.i2c
            .write(address, &[reg, value])
            .await
            .map_err(Error::I2c)
    }

    /// Read one register at the given I²C `address`.
    async fn read_register(&mut self, address: u8, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(address, &[reg], &mut buf)
            .await
            .map_err(Error::I2c)?;
        Ok(buf[0])
    }

    /// Write to an accelerometer register.
    pub async fn write_acc_reg(&mut self, reg: u8, value: u8) -> Result<(), Error<I2C::Error>> {
        let address = V::acc_address(self.sa0);
        self.write_register(address, reg, value).await
    }

    /// Read an accelerometer register.
    pub async fn read_acc_reg(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let address = V::acc_address(self.sa0);
        self.read_register(address, reg).await
    }

    /// Write to a magnetometer register.
    pub async fn write_mag_reg(&mut self, reg: u8, value: u8) -> Result<(), Error<I2C::Error>> {
        let address = V::mag_address(self.sa0);
        self.write_register(address, reg, value).await
    }

    /// Read a magnetometer register.
    pub async fn read_mag_reg(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
        let address = V::mag_address(self.sa0);
        self.read_register(address, reg).await
    }

    /// Apply the power-on default configuration for this variant.
    pub async fn enable_default(&mut self) -> Result<(), Error<I2C::Error>> {
        V::enable_default(self).await
    }

    /// Read the three accelerometer axes.
    pub async fn read_accel(&mut self) -> Result<Vector, Error<I2C::Error>> {
        let address = V::acc_address(self.sa0);
        let mut buf = [0u8; 6];
        // The MSB of the sub-address enables auto-increment for the burst read.
        self.i2c
            .write_read(address, &[reg_addr::OUT_X_L_A | 0x80], &mut buf)
            .await
            .map_err(Error::I2c)?;
        // Accelerometer output is X_L, X_H, Y_L, Y_H, Z_L, Z_H on every variant.
        Ok(Vector {
            x: i16::from_le_bytes([buf[0], buf[1]]),
            y: i16::from_le_bytes([buf[2], buf[3]]),
            z: i16::from_le_bytes([buf[4], buf[5]]),
        })
    }

    /// Read the three magnetometer axes.
    pub async fn read_mag(&mut self) -> Result<Vector, Error<I2C::Error>> {
        let address = V::mag_address(self.sa0);
        let mut buf = [0u8; 6];
        self.i2c
            .write_read(address, &[V::MAG_OUT_START], &mut buf)
            .await
            .map_err(Error::I2c)?;
        Ok(V::parse_mag(&buf))
    }

    /// Read both the accelerometer and magnetometer.
    pub async fn read(&mut self) -> Result<(Vector, Vector), Error<I2C::Error>> {
        Ok((self.read_accel().await?, self.read_mag().await?))
    }

    /// Read-modify-write the `mask` bits of an accelerometer register, leaving
    /// the other bits untouched.
    async fn modify_acc_reg(
        &mut self,
        reg: u8,
        mask: u8,
        bits: u8,
    ) -> Result<(), Error<I2C::Error>> {
        let current = self.read_acc_reg(reg).await?;
        self.write_acc_reg(reg, (current & !mask) | (bits & mask))
            .await
    }

    /// Read-modify-write the `mask` bits of a magnetometer register.
    async fn modify_mag_reg(
        &mut self,
        reg: u8,
        mask: u8,
        bits: u8,
    ) -> Result<(), Error<I2C::Error>> {
        let current = self.read_mag_reg(reg).await?;
        self.write_mag_reg(reg, (current & !mask) | (bits & mask))
            .await
    }
}

// Accelerometer full-scale, gated per variant: each chip accepts only its own
// range set, and the D uses a different register/field than the DLx trio.
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c> LSM303<I2C, Dlh> {
    /// Set the accelerometer full-scale range.
    pub async fn set_accel_scale(
        &mut self,
        scale: DlhDlmAccScale,
    ) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL_REG4_A, DlhDlmAccScale::MASK, scale.bits())
            .await
    }

    /// Set the accelerometer output data rate (also sets normal power mode).
    pub async fn set_accel_odr(&mut self, odr: DlhDlmAccOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL_REG1_A, DlhDlmAccOdr::MASK, odr.bits())
            .await
    }

    /// Set the magnetometer output data rate (`DO` in `CRA_REG_M`).
    pub async fn set_mag_odr(&mut self, odr: DlhDlmMagOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_mag_reg(reg_addr::CRA_REG_M, DlhDlmMagOdr::MASK, odr.bits())
            .await
    }

    /// Read back the currently configured accelerometer full-scale range.
    pub async fn read_accel_scale(&mut self) -> Result<DlhDlmAccScale, Error<I2C::Error>> {
        let reg = self.read_acc_reg(reg_addr::CTRL_REG4_A).await?;
        Ok(DlhDlmAccScale::from_reg(reg))
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c> LSM303<I2C, Dlm> {
    /// Set the accelerometer full-scale range.
    pub async fn set_accel_scale(
        &mut self,
        scale: DlhDlmAccScale,
    ) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL_REG4_A, DlhDlmAccScale::MASK, scale.bits())
            .await
    }

    /// Set the accelerometer output data rate (also sets normal power mode).
    pub async fn set_accel_odr(&mut self, odr: DlhDlmAccOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL_REG1_A, DlhDlmAccOdr::MASK, odr.bits())
            .await
    }

    /// Set the magnetometer output data rate (`DO` in `CRA_REG_M`).
    pub async fn set_mag_odr(&mut self, odr: DlhDlmMagOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_mag_reg(reg_addr::CRA_REG_M, DlhDlmMagOdr::MASK, odr.bits())
            .await
    }

    /// Read back the currently configured accelerometer full-scale range.
    pub async fn read_accel_scale(&mut self) -> Result<DlhDlmAccScale, Error<I2C::Error>> {
        let reg = self.read_acc_reg(reg_addr::CTRL_REG4_A).await?;
        Ok(DlhDlmAccScale::from_reg(reg))
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c> LSM303<I2C, Dlhc> {
    /// Set the accelerometer full-scale range.
    pub async fn set_accel_scale(&mut self, scale: DlhcAccScale) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL_REG4_A, DlhcAccScale::MASK, scale.bits())
            .await
    }

    /// Set the accelerometer output data rate.
    pub async fn set_accel_odr(&mut self, odr: DlhcAccOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL_REG1_A, DlhcAccOdr::MASK, odr.bits())
            .await
    }

    /// Set the magnetometer output data rate, including the DLHC-only 220 Hz.
    pub async fn set_mag_odr(&mut self, odr: DlhcMagOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_mag_reg(reg_addr::CRA_REG_M, DlhcMagOdr::MASK, odr.bits())
            .await
    }

    /// Read back the currently configured accelerometer full-scale range.
    pub async fn read_accel_scale(&mut self) -> Result<DlhcAccScale, Error<I2C::Error>> {
        let reg = self.read_acc_reg(reg_addr::CTRL_REG4_A).await?;
        Ok(DlhcAccScale::from_reg(reg))
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c> LSM303<I2C, D> {
    /// Set the accelerometer full-scale range (`AFS` in `CTRL2`).
    pub async fn set_accel_scale(&mut self, scale: DAccScale) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL2, DAccScale::MASK, scale.bits())
            .await
    }

    /// Set the accelerometer output data rate (`AODR` in `CTRL1`).
    pub async fn set_accel_odr(&mut self, odr: DAccOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_acc_reg(reg_addr::CTRL1, DAccOdr::MASK, odr.bits())
            .await
    }

    /// Set the magnetometer full-scale range (`MFS` in `CTRL6`).
    pub async fn set_mag_scale(&mut self, scale: DMagScale) -> Result<(), Error<I2C::Error>> {
        self.modify_mag_reg(reg_addr::CTRL6, DMagScale::MASK, scale.bits())
            .await
    }

    /// Set the magnetometer output data rate (`M_ODR` in `CTRL5`).
    pub async fn set_mag_odr(&mut self, odr: DMagOdr) -> Result<(), Error<I2C::Error>> {
        self.modify_mag_reg(reg_addr::CTRL5, DMagOdr::MASK, odr.bits())
            .await
    }

    /// Read back the currently configured accelerometer full-scale range.
    pub async fn read_accel_scale(&mut self) -> Result<DAccScale, Error<I2C::Error>> {
        let reg = self.read_acc_reg(reg_addr::CTRL2).await?;
        Ok(DAccScale::from_reg(reg))
    }
}

// Magnetometer full-scale, shared by the DLH/DLM/DLHC (identical `GN` table).
// Mag ODR is *not* shared: the DLHC supports 220 Hz that the DLH/DLM lack, so
// its setter lives on the per-variant blocks below.
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c, V: Dlx> LSM303<I2C, V> {
    /// Set the magnetometer full-scale range.
    pub async fn set_mag_scale(&mut self, scale: DlxMagScale) -> Result<(), Error<I2C::Error>> {
        self.modify_mag_reg(reg_addr::CRB_REG_M, DlxMagScale::MASK, scale.bits())
            .await
    }
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
// The returned futures are only awaited inside this crate / on a single-core
// embassy executor, so `Send` bounds are unnecessary. Same choice as
// `embedded-hal-async` makes for its own traits.
#[allow(async_fn_in_trait)]
pub trait HasTemperature<I2C: I2c> {
    async fn read_temperature(&mut self) -> Result<u16, Error<I2C::Error>>;
}
#[maybe_async_cfg::maybe(sync(feature = "sync", keep_self), async(feature = "async", keep_self))]
impl<I2C: I2c> HasTemperature<I2C> for LSM303<I2C, D> {
    async fn read_temperature(&mut self) -> Result<u16, Error<I2C::Error>> {
        let temp_out_l = self.read_acc_reg(TEMP_OUT_L).await?;
        let temp_out_h = self.read_acc_reg(TEMP_OUT_H).await?;
        Ok(u16::from_le_bytes([temp_out_l, temp_out_h]))
    }
}
