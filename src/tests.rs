//! Unit tests, run against a mock I²C bus.
//!
//! Expected register values come from the ST datasheets:
//! LSM303DLH (Doc ID 16941), LSM303DLM (Doc ID 018725), LSM303DLHC and
//! LSM303D. Table numbers are cited next to each check.
//!
//! The same tests run under both the `async` and the `sync` feature; `run!`
//! drives the future to completion in async mode and is a no-op in sync mode.

extern crate std;

use std::vec;
use std::vec::Vec;

use embedded_hal_mock::eh1::i2c::{Mock, Transaction as T};
#[cfg(feature = "sync")]
use embedded_hal::i2c::{ErrorKind, NoAcknowledgeSource};
#[cfg(feature = "async")]
use embedded_hal_async::i2c::{ErrorKind, NoAcknowledgeSource};

use crate::*;

#[cfg(feature = "async")]
macro_rules! run {
    ($e:expr) => {
        pollster::block_on($e)
    };
}

#[cfg(feature = "sync")]
macro_rules! run {
    ($e:expr) => {
        $e
    };
}

// I²C addresses (DLH Doc ID 16941 §7.1.1, DLHC/D datasheets "I2C operation").
const D_HIGH: u8 = 0x1D;
const D_LOW: u8 = 0x1E;
const ACC_HIGH: u8 = 0x19;
const ACC_LOW: u8 = 0x18;
const MAG: u8 = 0x1E;

const NACK: ErrorKind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);

/// A register read that the device does not ACK.
fn nack(addr: u8, reg: u8) -> T {
    T::write_read(addr, vec![reg], vec![0]).with_error(NACK)
}

/// A single-register read.
fn rd(addr: u8, reg: u8, value: u8) -> T {
    T::write_read(addr, vec![reg], vec![value])
}

/// A single-register write.
fn wr(addr: u8, reg: u8, value: u8) -> T {
    T::write(addr, vec![reg, value])
}

// ---------------------------------------------------------------------------
// Register encodings, checked against the datasheet tables.
// ---------------------------------------------------------------------------

/// Every encoding must stay inside its field mask.
fn assert_in_mask(bits: &[u8], mask: u8) {
    for b in bits {
        assert_eq!(b & !mask, 0, "bits {b:#010b} outside mask {mask:#010b}");
    }
}

#[test]
fn dlx_mag_scale_matches_gain_table() {
    // GN[2:0] in CRB_REG_M bits 7:5 (DLH Table 62, DLM Table 58, DLHC Table 75).
    use DlxMagScale::*;
    let table = [
        (G1_3, 0b001),
        (G1_9, 0b010),
        (G2_5, 0b011),
        (G4_0, 0b100),
        (G4_7, 0b101),
        (G5_6, 0b110),
        (G8_1, 0b111),
    ];
    for (scale, gn) in table {
        assert_eq!(scale.bits(), gn << 5, "{scale:?}");
    }
    assert_eq!(DlxMagScale::MASK, 0b1110_0000);
    assert_in_mask(&table.map(|(s, _)| s.bits()), DlxMagScale::MASK);
}

#[test]
fn d_mag_scale_matches_table_50() {
    // MFS[1:0] in CTRL6 bits 6:5 (LSM303D Table 48/50).
    use DMagScale::*;
    let table = [(G2, 0b00), (G4, 0b01), (G8, 0b10), (G12, 0b11)];
    for (scale, mfs) in table {
        assert_eq!(scale.bits(), mfs << 5, "{scale:?}");
    }
    assert_eq!(DMagScale::MASK, 0b0110_0000);
}

#[test]
fn dlh_dlm_acc_scale_matches_ctrl_reg4_a() {
    // FS[1:0] in CTRL_REG4_A bits 5:4: 00 = 2 g, 01 = 4 g, 11 = 8 g
    // (DLH Table 30, DLM Table 28).
    use DlhDlmAccScale::*;
    let table = [(G2, 0b00), (G4, 0b01), (G8, 0b11)];
    for (scale, fs) in table {
        assert_eq!(scale.bits(), fs << 4, "{scale:?}");
        assert_eq!(DlhDlmAccScale::from_reg(scale.bits()), scale);
        // Other bits in the register must not affect decoding.
        assert_eq!(DlhDlmAccScale::from_reg(scale.bits() | 0b1100_1111), scale);
    }
    assert_eq!(DlhDlmAccScale::MASK, 0b0011_0000);
}

#[test]
fn dlhc_acc_scale_matches_ctrl_reg4_a() {
    // FS[1:0] in CTRL_REG4_A bits 5:4: 00 = 2 g, 01 = 4 g, 10 = 8 g, 11 = 16 g
    // (DLHC Table 27).
    use DlhcAccScale::*;
    let table = [(G2, 0b00), (G4, 0b01), (G8, 0b10), (G16, 0b11)];
    for (scale, fs) in table {
        assert_eq!(scale.bits(), fs << 4, "{scale:?}");
        assert_eq!(DlhcAccScale::from_reg(scale.bits()), scale);
        assert_eq!(DlhcAccScale::from_reg(scale.bits() | 0b1100_1111), scale);
    }
    assert_eq!(DlhcAccScale::MASK, 0b0011_0000);
}

#[test]
fn d_acc_scale_matches_table_40() {
    // AFS[2:0] in CTRL2 bits 5:3 (LSM303D Table 37/40).
    use DAccScale::*;
    let table = [(G2, 0b000), (G4, 0b001), (G6, 0b010), (G8, 0b011), (G16, 0b100)];
    for (scale, afs) in table {
        assert_eq!(scale.bits(), afs << 3, "{scale:?}");
        assert_eq!(DAccScale::from_reg(scale.bits()), scale);
        assert_eq!(DAccScale::from_reg(scale.bits() | 0b1100_0111), scale);
    }
    assert_eq!(DAccScale::MASK, 0b0011_1000);
}

#[test]
fn dlh_dlm_acc_odr_matches_tables_20_21() {
    // CTRL_REG1_A: PM[2:0] bits 7:5, DR[1:0] bits 4:3. Rates are only
    // produced in normal mode (PM = 001); PM = 000 is power-down
    // (DLH Tables 18-21, DLM Tables 16-19).
    use DlhDlmAccOdr::*;
    let table = [
        (PowerDown, 0b000, 0b00),
        (Hz50, 0b001, 0b00),
        (Hz100, 0b001, 0b01),
        (Hz400, 0b001, 0b10),
        (Hz1000, 0b001, 0b11),
    ];
    for (odr, pm, dr) in table {
        assert_eq!(odr.bits(), (pm << 5) | (dr << 3), "{odr:?}");
    }
    // The axis-enable bits (2:0) must be left alone.
    assert_eq!(DlhDlmAccOdr::MASK, 0b1111_1000);
}

#[test]
fn dlhc_acc_odr_matches_table_20() {
    // ODR[3:0] in CTRL_REG1_A bits 7:4 (DLHC Table 18/20).
    use DlhcAccOdr::*;
    let table = [
        (PowerDown, 0b0000),
        (Hz1, 0b0001),
        (Hz10, 0b0010),
        (Hz25, 0b0011),
        (Hz50, 0b0100),
        (Hz100, 0b0101),
        (Hz200, 0b0110),
        (Hz400, 0b0111),
    ];
    for (odr, code) in table {
        assert_eq!(odr.bits(), code << 4, "{odr:?}");
    }
    assert_eq!(DlhcAccOdr::MASK, 0b1111_0000);
}

#[test]
fn d_acc_odr_matches_table_36() {
    // AODR[3:0] in CTRL1 bits 7:4 (LSM303D Table 34/36).
    use DAccOdr::*;
    let table = [
        (PowerDown, 0b0000),
        (Hz3_125, 0b0001),
        (Hz6_25, 0b0010),
        (Hz12_5, 0b0011),
        (Hz25, 0b0100),
        (Hz50, 0b0101),
        (Hz100, 0b0110),
        (Hz200, 0b0111),
        (Hz400, 0b1000),
        (Hz800, 0b1001),
        (Hz1600, 0b1010),
    ];
    for (odr, code) in table {
        assert_eq!(odr.bits(), code << 4, "{odr:?}");
    }
    assert_eq!(DAccOdr::MASK, 0b1111_0000);
}

#[test]
fn dlh_mag_odr_matches_table_58() {
    // DO[2:0] in CRA_REG_M bits 4:2; 111 is "not used" (DLH Table 58).
    use DlhMagOdr::*;
    let table = [
        (Hz0_75, 0b000),
        (Hz1_5, 0b001),
        (Hz3, 0b010),
        (Hz7_5, 0b011),
        (Hz15, 0b100),
        (Hz30, 0b101),
        (Hz75, 0b110),
    ];
    for (odr, code) in table {
        assert_eq!(odr.bits(), code << 2, "{odr:?}");
    }
    assert_eq!(DlhMagOdr::MASK, 0b0001_1100);
}

#[test]
fn dlm_dlhc_mag_odr_matches_do_table() {
    // DO[2:0] in CRA_REG_M bits 4:2, including 220 Hz at 111
    // (DLM Table 56, DLHC Table 72).
    use DlmDlhcMagOdr::*;
    let table = [
        (Hz0_75, 0b000),
        (Hz1_5, 0b001),
        (Hz3, 0b010),
        (Hz7_5, 0b011),
        (Hz15, 0b100),
        (Hz30, 0b101),
        (Hz75, 0b110),
        (Hz220, 0b111),
    ];
    for (odr, code) in table {
        assert_eq!(odr.bits(), code << 2, "{odr:?}");
    }
    // Must not touch TEMP_EN (bit 7).
    assert_eq!(DlmDlhcMagOdr::MASK, 0b0001_1100);
}

#[test]
fn d_mag_odr_matches_table_47() {
    // M_ODR[2:0] in CTRL5 bits 4:2 (LSM303D Table 45/47).
    use DMagOdr::*;
    let table = [
        (Hz3_125, 0b000),
        (Hz6_25, 0b001),
        (Hz12_5, 0b010),
        (Hz25, 0b011),
        (Hz50, 0b100),
        (Hz100, 0b101),
    ];
    for (odr, code) in table {
        assert_eq!(odr.bits(), code << 2, "{odr:?}");
    }
    // Must not touch TEMP_EN / M_RES (bits 7:5) or LIR (bits 1:0).
    assert_eq!(DMagOdr::MASK, 0b0001_1100);
}

// ---------------------------------------------------------------------------
// Addresses and magnetometer byte order.
// ---------------------------------------------------------------------------

#[test]
fn addresses() {
    // LSM303D: one address for both sensors, 0011101b (SA0 high) or 0011110b.
    assert_eq!(D::acc_address(SA0::High), D_HIGH);
    assert_eq!(D::acc_address(SA0::Low), D_LOW);
    assert_eq!(D::mag_address(SA0::High), D_HIGH);
    assert_eq!(D::mag_address(SA0::Low), D_LOW);

    // DLH/DLM: accelerometer 0011001b / 0011000b, magnetometer fixed 0011110b.
    for sa0 in [SA0::High, SA0::Low] {
        assert_eq!(Dlh::mag_address(sa0), MAG);
        assert_eq!(Dlm::mag_address(sa0), MAG);
        assert_eq!(Dlhc::mag_address(sa0), MAG);
        // DLHC accelerometer address is fixed.
        assert_eq!(Dlhc::acc_address(sa0), ACC_HIGH);
    }
    assert_eq!(Dlh::acc_address(SA0::High), ACC_HIGH);
    assert_eq!(Dlh::acc_address(SA0::Low), ACC_LOW);
    assert_eq!(Dlm::acc_address(SA0::High), ACC_HIGH);
    assert_eq!(Dlm::acc_address(SA0::Low), ACC_LOW);
}

#[test]
fn parse_mag_byte_orders() {
    let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];

    // DLH: X_H, X_L, Y_H, Y_L, Z_H, Z_L (DLH Table 17, 03h-08h).
    assert_eq!(
        Dlh::parse_mag(&buf),
        Vector { x: 0x0102, y: 0x0304, z: 0x0506 }
    );
    // DLM / DLHC: X_H, X_L, Z_H, Z_L, Y_H, Y_L (DLM Table 15, DLHC Table 17).
    let xzy = Vector { x: 0x0102, z: 0x0304, y: 0x0506 };
    assert_eq!(Dlm::parse_mag(&buf), xzy);
    assert_eq!(Dlhc::parse_mag(&buf), xzy);
    // D: X_L, X_H, Y_L, Y_H, Z_L, Z_H (LSM303D Table 16, 08h-0Dh).
    assert_eq!(
        D::parse_mag(&buf),
        Vector { x: 0x0201, y: 0x0403, z: 0x0605 }
    );
    // Two's complement.
    assert_eq!(D::parse_mag(&[0xFF, 0xFF, 0x00, 0x80, 0xFF, 0x7F]), Vector {
        x: -1,
        y: i16::MIN,
        z: i16::MAX,
    });
}

#[test]
fn mag_burst_start_registers() {
    // DLx magnetometers auto-increment on their own; start at OUT_X_H_M (03h).
    assert_eq!(Dlh::MAG_OUT_START, 0x03);
    assert_eq!(Dlm::MAG_OUT_START, 0x03);
    assert_eq!(Dlhc::MAG_OUT_START, 0x03);
    // D needs the SUB MSb set to auto-increment; OUT_X_L_M is 08h.
    assert_eq!(D::MAG_OUT_START, 0x08 | 0x80);
}

// ---------------------------------------------------------------------------
// Reading data over the bus.
// ---------------------------------------------------------------------------

#[test]
fn read_accel_burst_reads_with_auto_increment() {
    // OUT_X_L_A (28h) with the SUB MSb set: 0xA8.
    let mut bus = Mock::new(&[T::write_read(
        D_HIGH,
        vec![0xA8],
        vec![0x01, 0x02, 0xFF, 0xFF, 0x00, 0x80],
    )]);
    let mut dev = Lsm303::<_, D>::new(bus.clone(), SA0::High);
    let v = run!(dev.read_accel()).unwrap();
    assert_eq!(v, Vector { x: 0x0201, y: -1, z: i16::MIN });
    bus.done();
}

#[test]
fn read_accel_uses_variant_address() {
    let data = vec![0; 6];
    let mut bus = Mock::new(&[
        T::write_read(ACC_LOW, vec![0xA8], data.clone()),
        T::write_read(ACC_HIGH, vec![0xA8], data.clone()),
        T::write_read(ACC_HIGH, vec![0xA8], data.clone()),
        T::write_read(D_LOW, vec![0xA8], data),
    ]);
    run!(Lsm303::<_, Dlm>::new(bus.clone(), SA0::Low).read_accel()).unwrap();
    run!(Lsm303::<_, Dlh>::new(bus.clone(), SA0::High).read_accel()).unwrap();
    // DLHC ignores SA0.
    run!(Lsm303::<_, Dlhc>::new(bus.clone(), SA0::Low).read_accel()).unwrap();
    run!(Lsm303::<_, D>::new(bus.clone(), SA0::Low).read_accel()).unwrap();
    bus.done();
}

#[test]
fn read_mag_per_variant() {
    let raw = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
    let mut bus = Mock::new(&[
        T::write_read(MAG, vec![0x03], raw.clone()),
        T::write_read(MAG, vec![0x03], raw.clone()),
        T::write_read(MAG, vec![0x03], raw.clone()),
        T::write_read(D_HIGH, vec![0x88], raw),
    ]);
    let dlh = run!(Lsm303::<_, Dlh>::new(bus.clone(), SA0::High).read_mag()).unwrap();
    let dlm = run!(Lsm303::<_, Dlm>::new(bus.clone(), SA0::High).read_mag()).unwrap();
    let dlhc = run!(Lsm303::<_, Dlhc>::new(bus.clone(), SA0::High).read_mag()).unwrap();
    let d = run!(Lsm303::<_, D>::new(bus.clone(), SA0::High).read_mag()).unwrap();
    assert_eq!(dlh, Vector { x: 0x0102, y: 0x0304, z: 0x0506 });
    assert_eq!(dlm, Vector { x: 0x0102, z: 0x0304, y: 0x0506 });
    assert_eq!(dlhc, dlm);
    assert_eq!(d, Vector { x: 0x0201, y: 0x0403, z: 0x0605 });
    bus.done();
}

#[test]
fn read_returns_accel_then_mag() {
    let mut bus = Mock::new(&[
        T::write_read(ACC_HIGH, vec![0xA8], vec![1, 0, 2, 0, 3, 0]),
        T::write_read(MAG, vec![0x03], vec![0, 4, 0, 6, 0, 5]),
    ]);
    let mut dev = Lsm303::<_, Dlhc>::new(bus.clone(), SA0::High);
    let (acc, mag) = run!(dev.read()).unwrap();
    assert_eq!(acc, Vector { x: 1, y: 2, z: 3 });
    assert_eq!(mag, Vector { x: 4, y: 5, z: 6 });
    bus.done();
}

#[test]
fn bus_errors_are_propagated() {
    let mut bus = Mock::new(&[
        T::write_read(D_HIGH, vec![0xA8], vec![0; 6]).with_error(ErrorKind::Bus)
    ]);
    let mut dev = Lsm303::<_, D>::new(bus.clone(), SA0::High);
    assert!(matches!(
        run!(dev.read_accel()),
        Err(Error::I2c(ErrorKind::Bus))
    ));
    bus.done();
}

#[test]
fn raw_register_access() {
    let mut bus = Mock::new(&[
        wr(ACC_HIGH, 0x20, 0x57),
        rd(ACC_HIGH, 0x20, 0x57),
        wr(MAG, 0x02, 0x00),
        rd(MAG, 0x02, 0x03),
    ]);
    let mut dev = Lsm303::<_, Dlm>::new(bus.clone(), SA0::High);
    run!(dev.write_acc_reg(0x20, 0x57)).unwrap();
    assert_eq!(run!(dev.read_acc_reg(0x20)).unwrap(), 0x57);
    run!(dev.write_mag_reg(0x02, 0x00)).unwrap();
    assert_eq!(run!(dev.read_mag_reg(0x02)).unwrap(), 0x03);
    bus.done();
}

#[test]
fn release_returns_the_bus() {
    let bus = Mock::new(&[rd(D_HIGH, WHO_AM_I, 0x49)]);
    let dev = Lsm303::<_, D>::new(bus, SA0::High);
    // The released bus is the same mock: it still holds the unused expectation.
    let mut again = Lsm303::<_, D>::new(dev.release(), SA0::High);
    assert_eq!(run!(again.read_acc_reg(WHO_AM_I)).unwrap(), 0x49);
    again.release().done();
}

// ---------------------------------------------------------------------------
// enable_default: register values checked field by field against the tables.
// ---------------------------------------------------------------------------

#[test]
fn enable_default_d() {
    let mut bus = Mock::new(&[
        wr(D_HIGH, 0x21, 0x00), // CTRL2: AFS = 000 (2 g)
        wr(D_HIGH, 0x20, 0x57), // CTRL1: AODR = 0101 (50 Hz), axes on
        wr(D_HIGH, 0x24, 0x64), // CTRL5: M_RES = 11, M_ODR = 001 (6.25 Hz)
        wr(D_HIGH, 0x25, 0x20), // CTRL6: MFS = 01 (4 gauss)
        wr(D_HIGH, 0x26, 0x00), // CTRL7: MD = 00 (continuous)
    ]);
    run!(Lsm303::<_, D>::new(bus.clone(), SA0::High).enable_default()).unwrap();
    bus.done();
}

#[test]
fn enable_default_dlhc() {
    let mut bus = Mock::new(&[
        wr(ACC_HIGH, 0x23, 0x08), // CTRL_REG4_A: FS = 00 (2 g), HR = 1
        wr(ACC_HIGH, 0x20, 0x47), // CTRL_REG1_A: ODR = 0100 (50 Hz), axes on
        wr(MAG, 0x00, 0x0C),      // CRA_REG_M: DO = 011 (7.5 Hz)
        wr(MAG, 0x01, 0x20),      // CRB_REG_M: GN = 001 (1.3 gauss)
        wr(MAG, 0x02, 0x00),      // MR_REG_M: continuous
    ]);
    run!(Lsm303::<_, Dlhc>::new(bus.clone(), SA0::High).enable_default()).unwrap();
    bus.done();
}

#[test]
fn enable_default_dlm_and_dlh() {
    let seq = |acc| {
        [
            wr(acc, 0x23, 0x00), // CTRL_REG4_A: FS = 00 (2 g)
            wr(acc, 0x20, 0x27), // CTRL_REG1_A: PM = 001, DR = 00 (50 Hz), axes on
            wr(MAG, 0x00, 0x0C), // CRA_REG_M: DO = 011 (7.5 Hz)
            wr(MAG, 0x01, 0x20), // CRB_REG_M: GN = 001 (1.3 gauss)
            wr(MAG, 0x02, 0x00), // MR_REG_M: continuous
        ]
    };
    let expected: Vec<T> = seq(ACC_LOW).into_iter().chain(seq(ACC_HIGH)).collect();
    let mut bus = Mock::new(&expected);
    run!(Lsm303::<_, Dlm>::new(bus.clone(), SA0::Low).enable_default()).unwrap();
    run!(Lsm303::<_, Dlh>::new(bus.clone(), SA0::High).enable_default()).unwrap();
    bus.done();
}

// ---------------------------------------------------------------------------
// Setters: read-modify-write only the field they own.
// ---------------------------------------------------------------------------

#[test]
fn d_setters_preserve_other_bits() {
    let mut bus = Mock::new(&[
        // CTRL2 = ABW 11 | AFS 000 | SIM 1 -> AFS = 100 (16 g)
        rd(D_HIGH, 0x21, 0b1100_0001),
        wr(D_HIGH, 0x21, 0b1110_0001),
        // CTRL1 = AODR 0101 | BDU 1 | axes 111 -> AODR = 1010 (1600 Hz)
        rd(D_HIGH, 0x20, 0b0101_1111),
        wr(D_HIGH, 0x20, 0b1010_1111),
        // CTRL6 = MFS 01 -> MFS = 11 (12 gauss)
        rd(D_HIGH, 0x25, 0b0010_0000),
        wr(D_HIGH, 0x25, 0b0110_0000),
        // CTRL5 = TEMP_EN 1 | M_RES 11 | M_ODR 001 | LIR 11 -> M_ODR = 101 (100 Hz)
        rd(D_HIGH, 0x24, 0b1110_0111),
        wr(D_HIGH, 0x24, 0b1111_0111),
        // read back AFS = 010 (6 g)
        rd(D_HIGH, 0x21, 0b1101_0001),
    ]);
    let mut dev = Lsm303::<_, D>::new(bus.clone(), SA0::High);
    run!(dev.set_accel_scale(DAccScale::G16)).unwrap();
    run!(dev.set_accel_odr(DAccOdr::Hz1600)).unwrap();
    run!(dev.set_mag_scale(DMagScale::G12)).unwrap();
    run!(dev.set_mag_odr(DMagOdr::Hz100)).unwrap();
    assert_eq!(run!(dev.read_accel_scale()).unwrap(), DAccScale::G6);
    bus.done();
}

#[test]
fn dlhc_setters_preserve_other_bits() {
    let mut bus = Mock::new(&[
        // CTRL_REG4_A = BDU 1 | FS 00 | HR 1 -> FS = 10 (8 g)
        rd(ACC_HIGH, 0x23, 0b1000_1000),
        wr(ACC_HIGH, 0x23, 0b1010_1000),
        // CTRL_REG1_A = ODR 0100 | LPen 1 | axes 111 -> ODR = 0111 (400 Hz)
        rd(ACC_HIGH, 0x20, 0b0100_1111),
        wr(ACC_HIGH, 0x20, 0b0111_1111),
        // CRA_REG_M = TEMP_EN 1 | DO 011 -> DO = 111 (220 Hz)
        rd(MAG, 0x00, 0b1000_1100),
        wr(MAG, 0x00, 0b1001_1100),
        // CRB_REG_M = GN 001 -> GN = 111 (8.1 gauss)
        rd(MAG, 0x01, 0b0010_0000),
        wr(MAG, 0x01, 0b1110_0000),
        // read back FS = 11 (16 g)
        rd(ACC_HIGH, 0x23, 0b0011_1000),
    ]);
    let mut dev = Lsm303::<_, Dlhc>::new(bus.clone(), SA0::High);
    run!(dev.set_accel_scale(DlhcAccScale::G8)).unwrap();
    run!(dev.set_accel_odr(DlhcAccOdr::Hz400)).unwrap();
    run!(dev.set_mag_odr(DlmDlhcMagOdr::Hz220)).unwrap();
    run!(dev.set_mag_scale(DlxMagScale::G8_1)).unwrap();
    assert_eq!(run!(dev.read_accel_scale()).unwrap(), DlhcAccScale::G16);
    bus.done();
}

#[test]
fn dlm_setters_preserve_other_bits() {
    let mut bus = Mock::new(&[
        // CTRL_REG4_A = BDU 1 | FS 00 -> FS = 11 (8 g)
        rd(ACC_LOW, 0x23, 0b1000_0000),
        wr(ACC_LOW, 0x23, 0b1011_0000),
        // CTRL_REG1_A = PM 010 (low-power) | DR 11 | axes 101 -> normal 100 Hz
        rd(ACC_LOW, 0x20, 0b0101_1101),
        wr(ACC_LOW, 0x20, 0b0010_1101),
        // CRA_REG_M = DO 100 -> DO = 111 (220 Hz, DLM Table 56)
        rd(MAG, 0x00, 0b0001_0000),
        wr(MAG, 0x00, 0b0001_1100),
        // CRB_REG_M = GN 001 -> GN = 100 (4.0 gauss)
        rd(MAG, 0x01, 0b0010_0000),
        wr(MAG, 0x01, 0b1000_0000),
        // read back FS = 01 (4 g)
        rd(ACC_LOW, 0x23, 0b0001_0000),
    ]);
    let mut dev = Lsm303::<_, Dlm>::new(bus.clone(), SA0::Low);
    run!(dev.set_accel_scale(DlhDlmAccScale::G8)).unwrap();
    run!(dev.set_accel_odr(DlhDlmAccOdr::Hz100)).unwrap();
    run!(dev.set_mag_odr(DlmDlhcMagOdr::Hz220)).unwrap();
    run!(dev.set_mag_scale(DlxMagScale::G4_0)).unwrap();
    assert_eq!(run!(dev.read_accel_scale()).unwrap(), DlhDlmAccScale::G4);
    bus.done();
}

#[test]
fn dlh_setters_preserve_other_bits() {
    let mut bus = Mock::new(&[
        // CTRL_REG4_A = FS 11 | STsign 1 -> FS = 00 (2 g)
        rd(ACC_HIGH, 0x23, 0b0011_1000),
        wr(ACC_HIGH, 0x23, 0b0000_1000),
        // CTRL_REG1_A = normal 50 Hz, axes on -> power-down, axes kept
        rd(ACC_HIGH, 0x20, 0b0010_0111),
        wr(ACC_HIGH, 0x20, 0b0000_0111),
        // CRA_REG_M = DO 100 | MS 01 -> DO = 000 (0.75 Hz), MS kept
        rd(MAG, 0x00, 0b0001_0001),
        wr(MAG, 0x00, 0b0000_0001),
        // read back FS = 11 (8 g)
        rd(ACC_HIGH, 0x23, 0b0011_0000),
    ]);
    let mut dev = Lsm303::<_, Dlh>::new(bus.clone(), SA0::High);
    run!(dev.set_accel_scale(DlhDlmAccScale::G2)).unwrap();
    run!(dev.set_accel_odr(DlhDlmAccOdr::PowerDown)).unwrap();
    run!(dev.set_mag_odr(DlhMagOdr::Hz0_75)).unwrap();
    assert_eq!(run!(dev.read_accel_scale()).unwrap(), DlhDlmAccScale::G8);
    bus.done();
}

// ---------------------------------------------------------------------------
// Temperature (LSM303D §4.2 / §8.1: 12-bit two's complement, right-justified).
// ---------------------------------------------------------------------------

fn read_temp(l: u8, h: u8) -> i16 {
    let mut bus = Mock::new(&[rd(D_HIGH, 0x05, l), rd(D_HIGH, 0x06, h)]);
    let mut dev = Lsm303::<_, D>::new(bus.clone(), SA0::High);
    let t = run!(dev.read_temperature()).unwrap();
    bus.done();
    t
}

#[test]
fn temperature_is_signed_12_bit() {
    assert_eq!(read_temp(0x00, 0x00), 0);
    assert_eq!(read_temp(0x10, 0x00), 16); // +2 °C at 8 LSB/°C
    assert_eq!(read_temp(0xFF, 0x07), 2047); // max positive
    assert_eq!(read_temp(0xF0, 0x0F), -16); // -2 °C
    assert_eq!(read_temp(0x00, 0x08), -2048); // min negative
    // Bits above bit 11 are ignored whatever they hold.
    assert_eq!(read_temp(0xF0, 0xFF), -16);
    assert_eq!(read_temp(0x10, 0xF0), 16);
}

// ---------------------------------------------------------------------------
// Detection (ported from Pololu's LSM303::init).
// ---------------------------------------------------------------------------

const WHO_AM_I: u8 = 0x0F;
const CTRL_REG1_A: u8 = 0x20;

fn detect(bus: &mut Mock, device: Option<Device>, sa0: Option<SA0>) -> Result<AnyLsm303, Error<ErrorKind>> {
    run!(AnyLsm303::detect(bus, device, sa0))
}

#[test]
fn detect_d_sa0_high() {
    let mut bus = Mock::new(&[rd(D_HIGH, WHO_AM_I, 0x49)]);
    let found = detect(&mut bus, None, None).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::D, SA0::High));
    bus.done();
}

#[test]
fn detect_d_sa0_low() {
    let mut bus = Mock::new(&[nack(D_HIGH, WHO_AM_I), rd(D_LOW, WHO_AM_I, 0x49)]);
    let found = detect(&mut bus, None, None).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::D, SA0::Low));
    bus.done();
}

#[test]
fn detect_dlhc() {
    // 0x1E answers (it's the DLx magnetometer) but not with the D's ID; the
    // accelerometer ACKs at 0x19 and the magnetometer reports the DLM ID,
    // which on an 0x19 accelerometer means DLHC.
    let mut bus = Mock::new(&[
        nack(D_HIGH, WHO_AM_I),
        rd(D_LOW, WHO_AM_I, 0x3C),
        rd(ACC_HIGH, CTRL_REG1_A, 0x07),
        rd(MAG, WHO_AM_I, 0x3C),
    ]);
    let found = detect(&mut bus, None, None).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::Dlhc, SA0::High));
    bus.done();
}

#[test]
fn detect_dlm_sa0_low() {
    let mut bus = Mock::new(&[
        nack(D_HIGH, WHO_AM_I),
        rd(D_LOW, WHO_AM_I, 0x3C),
        nack(ACC_HIGH, CTRL_REG1_A),
        rd(ACC_LOW, CTRL_REG1_A, 0x07),
        rd(MAG, WHO_AM_I, 0x3C),
    ]);
    let found = detect(&mut bus, None, None).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::Dlm, SA0::Low));
    bus.done();
}

#[test]
fn detect_dlh() {
    // The DLH magnetometer has no WHO_AM_I_M, so it doesn't return 0x3C.
    let mut bus = Mock::new(&[
        nack(D_HIGH, WHO_AM_I),
        rd(D_LOW, WHO_AM_I, 0x00),
        rd(ACC_HIGH, CTRL_REG1_A, 0x07),
        rd(MAG, WHO_AM_I, 0x00),
    ]);
    let found = detect(&mut bus, None, None).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::Dlh, SA0::High));
    bus.done();
}

#[test]
fn detect_dlm_with_sa0_high_is_reported_as_dlhc() {
    // Known limitation inherited from Pololu (see `AnyLsm303::detect` docs):
    // a DLM at the SA0-high address looks exactly like a DLHC on the bus.
    let dlm_sa0_high = [
        nack(D_HIGH, WHO_AM_I),
        rd(D_LOW, WHO_AM_I, 0x3C),
        rd(ACC_HIGH, CTRL_REG1_A, 0x07),
        rd(MAG, WHO_AM_I, 0x3C),
    ];
    let mut bus = Mock::new(&dlm_sa0_high);
    let found = detect(&mut bus, None, None).unwrap();
    assert_eq!(found.device(), Device::Dlhc);
    bus.done();

    // Pinning the device avoids it.
    let mut bus = Mock::new(&dlm_sa0_high[..0]);
    let found = detect(&mut bus, Some(Device::Dlm), Some(SA0::High)).unwrap();
    assert_eq!(found.device(), Device::Dlm);
    bus.done();
}

#[test]
fn detect_nothing_on_the_bus() {
    let mut bus = Mock::new(&[
        nack(D_HIGH, WHO_AM_I),
        nack(D_LOW, WHO_AM_I),
        nack(ACC_HIGH, CTRL_REG1_A),
        nack(ACC_LOW, CTRL_REG1_A),
    ]);
    assert!(matches!(
        detect(&mut bus, None, None),
        Err(Error::DeviceNotFound)
    ));
    bus.done();
}

#[test]
fn detect_propagates_real_bus_errors() {
    let mut bus = Mock::new(&[T::write_read(D_HIGH, vec![WHO_AM_I], vec![0])
        .with_error(ErrorKind::ArbitrationLoss)]);
    assert!(matches!(
        detect(&mut bus, None, None),
        Err(Error::I2c(ErrorKind::ArbitrationLoss))
    ));
    bus.done();
}

#[test]
fn detect_fully_specified_does_not_touch_the_bus() {
    let mut bus = Mock::new(&[]);
    let found = detect(&mut bus, Some(Device::Dlm), Some(SA0::Low)).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::Dlm, SA0::Low));
    bus.done();
}

#[test]
fn detect_restricted_to_d() {
    // Only the two D addresses are probed.
    let mut bus = Mock::new(&[nack(D_HIGH, WHO_AM_I), nack(D_LOW, WHO_AM_I)]);
    assert!(matches!(
        detect(&mut bus, Some(Device::D), None),
        Err(Error::DeviceNotFound)
    ));
    bus.done();
}

#[test]
fn detect_restricted_to_sa0_low() {
    // SA0 high addresses are skipped.
    let mut bus = Mock::new(&[rd(D_LOW, WHO_AM_I, 0x49)]);
    let found = detect(&mut bus, None, Some(SA0::Low)).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::D, SA0::Low));
    bus.done();
}

#[test]
fn detect_pinned_dlx_device_is_kept() {
    // With the device pinned, the magnetometer ID check is skipped.
    let mut bus = Mock::new(&[rd(ACC_HIGH, CTRL_REG1_A, 0x07)]);
    let found = detect(&mut bus, Some(Device::Dlh), None).unwrap();
    assert_eq!((found.device(), found.sa0()), (Device::Dlh, SA0::High));
    bus.done();
}

#[test]
fn into_driver_checks_the_variant() {
    let mut bus = Mock::new(&[]);
    let found = detect(&mut bus, Some(Device::Dlhc), Some(SA0::High)).unwrap();
    let wrong: Result<Lsm303<_, D>, _> = found.into_driver(bus.clone());
    assert!(matches!(wrong, Err(Device::Dlhc)));

    let found = detect(&mut bus, Some(Device::Dlhc), Some(SA0::High)).unwrap();
    let right: Result<Lsm303<_, Dlhc>, _> = found.into_driver(bus.clone());
    assert!(right.is_ok());
    bus.done();
}
