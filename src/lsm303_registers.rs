#[allow(dead_code)]
pub mod reg_addr {
    pub const TEMP_OUT_L: u8 = 0x05; // D
    pub const TEMP_OUT_H: u8 = 0x06; // D

    pub const STATUS_M: u8 = 0x07; // D

    pub const INT_CTRL_M: u8 = 0x12; // D
    pub const INT_SRC_M: u8 = 0x13; // D
    pub const INT_THS_L_M: u8 = 0x14; // D
    pub const INT_THS_H_M: u8 = 0x15; // D

    pub const OFFSET_X_L_M: u8 = 0x16; // D
    pub const OFFSET_X_H_M: u8 = 0x17; // D
    pub const OFFSET_Y_L_M: u8 = 0x18; // D
    pub const OFFSET_Y_H_M: u8 = 0x19; // D
    pub const OFFSET_Z_L_M: u8 = 0x1A; // D
    pub const OFFSET_Z_H_M: u8 = 0x1B; // D
    pub const REFERENCE_X: u8 = 0x1C; // D
    pub const REFERENCE_Y: u8 = 0x1D; // D
    pub const REFERENCE_Z: u8 = 0x1E; // D

    pub const CTRL0: u8 = 0x1F; // D
    pub const CTRL1: u8 = 0x20; // D
    pub const CTRL_REG1_A: u8 = 0x20; // DLH, DLM, DLHC
    pub const CTRL2: u8 = 0x21; // D
    pub const CTRL_REG2_A: u8 = 0x21; // DLH, DLM, DLHC
    pub const CTRL3: u8 = 0x22; // D
    pub const CTRL_REG3_A: u8 = 0x22; // DLH, DLM, DLHC
    pub const CTRL4: u8 = 0x23; // D
    pub const CTRL_REG4_A: u8 = 0x23; // DLH, DLM, DLHC
    pub const CTRL5: u8 = 0x24; // D
    pub const CTRL_REG5_A: u8 = 0x24; // DLH, DLM, DLHC
    pub const CTRL6: u8 = 0x25; // D
    pub const CTRL_REG6_A: u8 = 0x25; // DLHC
    pub const HP_FILTER_RESET_A: u8 = 0x25; // DLH, DLM
    pub const CTRL7: u8 = 0x26; // D
    pub const REFERENCE_A: u8 = 0x26; // DLH, DLM, DLHC
    pub const STATUS_A: u8 = 0x27; // D
    pub const STATUS_REG_A: u8 = 0x27; // DLH, DLM, DLHC

    pub const OUT_X_L_A: u8 = 0x28;
    pub const OUT_X_H_A: u8 = 0x29;
    pub const OUT_Y_L_A: u8 = 0x2A;
    pub const OUT_Y_H_A: u8 = 0x2B;
    pub const OUT_Z_L_A: u8 = 0x2C;
    pub const OUT_Z_H_A: u8 = 0x2D;

    pub const FIFO_CTRL: u8 = 0x2E; // D
    pub const FIFO_CTRL_REG_A: u8 = 0x2E; // DLHC
    pub const FIFO_SRC: u8 = 0x2F; // D
    pub const FIFO_SRC_REG_A: u8 = 0x2F; // DLHC

    pub const IG_CFG1: u8 = 0x30; // D
    pub const INT1_CFG_A: u8 = 0x30; // DLH, DLM, DLHC
    pub const IG_SRC1: u8 = 0x31; // D
    pub const INT1_SRC_A: u8 = 0x31; // DLH, DLM, DLHC
    pub const IG_THS1: u8 = 0x32; // D
    pub const INT1_THS_A: u8 = 0x32; // DLH, DLM, DLHC
    pub const IG_DUR1: u8 = 0x33; // D
    pub const INT1_DURATION_A: u8 = 0x33; // DLH, DLM, DLHC
    pub const IG_CFG2: u8 = 0x34; // D
    pub const INT2_CFG_A: u8 = 0x34; // DLH, DLM, DLHC
    pub const IG_SRC2: u8 = 0x35; // D
    pub const INT2_SRC_A: u8 = 0x35; // DLH, DLM, DLHC
    pub const IG_THS2: u8 = 0x36; // D
    pub const INT2_THS_A: u8 = 0x36; // DLH, DLM, DLHC
    pub const IG_DUR2: u8 = 0x37; // D
    pub const INT2_DURATION_A: u8 = 0x37; // DLH, DLM, DLHC

    pub const CLICK_CFG: u8 = 0x38; // D
    pub const CLICK_CFG_A: u8 = 0x38; // DLHC
    pub const CLICK_SRC: u8 = 0x39; // D
    pub const CLICK_SRC_A: u8 = 0x39; // DLHC
    pub const CLICK_THS: u8 = 0x3A; // D
    pub const CLICK_THS_A: u8 = 0x3A; // DLHC
    pub const TIME_LIMIT: u8 = 0x3B; // D
    pub const TIME_LIMIT_A: u8 = 0x3B; // DLHC
    pub const TIME_LATENCY: u8 = 0x3C; // D
    pub const TIME_LATENCY_A: u8 = 0x3C; // DLHC
    pub const TIME_WINDOW: u8 = 0x3D; // D
    pub const TIME_WINDOW_A: u8 = 0x3D; // DLHC

    pub const ACT_THS: u8 = 0x3E; // D
    pub const ACT_DUR: u8 = 0x3F; // D

    pub const CRA_REG_M: u8 = 0x00; // DLH, DLM, DLHC
    pub const CRB_REG_M: u8 = 0x01; // DLH, DLM, DLHC
    pub const MR_REG_M: u8 = 0x02; // DLH, DLM, DLHC

    pub const SR_REG_M: u8 = 0x09; // DLH, DLM, DLHC
    pub const IRA_REG_M: u8 = 0x0A; // DLH, DLM, DLHC
    pub const IRB_REG_M: u8 = 0x0B; // DLH, DLM, DLHC
    pub const IRC_REG_M: u8 = 0x0C; // DLH, DLM, DLHC

    pub const WHO_AM_I: u8 = 0x0F; // D
    pub const WHO_AM_I_M: u8 = 0x0F; // DLM

    pub const TEMP_OUT_H_M: u8 = 0x31; // DLHC
    pub const TEMP_OUT_L_M: u8 = 0x32; // DLHC

    // Device-specific register addresses.
    pub const DLH_OUT_X_H_M: u8 = 0x03;
    pub const DLH_OUT_X_L_M: u8 = 0x04;
    pub const DLH_OUT_Y_H_M: u8 = 0x05;
    pub const DLH_OUT_Y_L_M: u8 = 0x06;
    pub const DLH_OUT_Z_H_M: u8 = 0x07;
    pub const DLH_OUT_Z_L_M: u8 = 0x08;

    pub const DLM_OUT_X_H_M: u8 = 0x03;
    pub const DLM_OUT_X_L_M: u8 = 0x04;
    pub const DLM_OUT_Z_H_M: u8 = 0x05;
    pub const DLM_OUT_Z_L_M: u8 = 0x06;
    pub const DLM_OUT_Y_H_M: u8 = 0x07;
    pub const DLM_OUT_Y_L_M: u8 = 0x08;

    pub const DLHC_OUT_X_H_M: u8 = 0x03;
    pub const DLHC_OUT_X_L_M: u8 = 0x04;
    pub const DLHC_OUT_Z_H_M: u8 = 0x05;
    pub const DLHC_OUT_Z_L_M: u8 = 0x06;
    pub const DLHC_OUT_Y_H_M: u8 = 0x07;
    pub const DLHC_OUT_Y_L_M: u8 = 0x08;

    pub const D_OUT_X_L_M: u8 = 0x08;
    pub const D_OUT_X_H_M: u8 = 0x09;
    pub const D_OUT_Y_L_M: u8 = 0x0A;
    pub const D_OUT_Y_H_M: u8 = 0x0B;
    pub const D_OUT_Z_L_M: u8 = 0x0C;
    pub const D_OUT_Z_H_M: u8 = 0x0D;
}
