//! Currently only supports the 8-bit SPI mode with dedicated DC pin.
//!
//! Currently only tested with the CFAP200200A0-154. The datasheet and sample
//! code for this part is known to be misleading, incomplete, and sometimes
//! outright wrong. Where possible the datasheet was checked against the
//! datasheet for the functionally equivalent replacement part, the
//! CFAP200200A1-154.

#![no_std]

extern crate embedded_hal;
#[macro_use]
extern crate nb;
extern crate volatile_register;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::digital::{OutputPin, InputPin};
use embedded_hal::spi::FullDuplex;

// TODO: const fn
pub fn width_pixels_to_bytes(x: u16) -> u8 {
    // round up when converted from pixels to bytes
    ((x + 7) >> 3) as u8
}

pub enum Preset {
    CFAP200200A0_154,
    CFAP200200A1_154,
}

pub struct ScreenBuilder {
    // TODO: const generics
    pub x_size: u16,
    pub y_size: u16,

    pub soft_start: [u8; 3],
    pub vcom: u8,
    pub dummy_line: u8,
    pub gate_line: u8,
    pub entry_mode: EntryMode,

    pub lut_full: [u8; 30],
    pub lut_part: [u8; 30],
}

impl ScreenBuilder {
    pub fn preset(preset: Preset) -> ScreenBuilder {
        match preset {
            Preset::CFAP200200A0_154 => ScreenBuilder {
                x_size: 200,
                y_size: 200,

                soft_start: SOFT_START_CFAP200200A0_154,
                vcom: VCOM_CFAP200200A0_154,
                dummy_line: DUMMY_LINE_CFAP200200A0_154,
                gate_line: GATE_LINE_CFAP200200A0_154,
                entry_mode: EntryMode::XIncrementYDecrement,

                lut_full: LUT_FULL_CFAP200200A0_154,
                lut_part: LUT_PART_CFAP200200A0_154,
            },
            Preset::CFAP200200A1_154 => ScreenBuilder {
                x_size: 200,
                y_size: 200,

                soft_start: SOFT_START_CFAP200200A1_154,
                vcom: VCOM_CFAP200200A1_154,
                dummy_line: DUMMY_LINE_CFAP200200A1_154,
                gate_line: GATE_LINE_CFAP200200A1_154,
                entry_mode: EntryMode::XIncrementYDecrement,

                lut_full: LUT_FULL_CFAP200200A1_154,
                lut_part: LUT_PART_CFAP200200A1_154,
            },
        }
    }

    pub fn new_screen<SPI, DC, CS, BUSY, RST, ERR, DELAY>(
        self,
        serial: SPI,
        dc: DC,
        cs: CS,
        busy: BUSY,
        reset: RST,
        delay: &mut DELAY,
    ) -> Result<Screen<SPI, DC, CS, BUSY, RST, ERR>, ERR>
    where
        SPI: FullDuplex<u8, Error = ERR>,
        DC: OutputPin,
        CS: OutputPin,
        BUSY: InputPin,
        RST: OutputPin,
        DELAY: DelayMs<u16>,
    {
        Screen::new(
            serial,
            dc,
            cs,
            busy,
            reset,
            self,
            delay,
        )
    }
}

pub enum EntryMode {
    XDecrementYDecrement = 0b00,
    XIncrementYDecrement = 0b01,
    XDecrementYIncrement = 0b10,
    XIncrementYIncrement = 0b11,
}

#[derive(Clone, Copy, Debug)]
pub enum Command {
    /// Data: A[7:0], {[0; 7], A[8]}, {[0;5], B[2:0]}
    ///
    /// A[8:0]: MUX
    /// Sets the number of gates.
    /// MUX = A[8:0] + 1, POR = 0x12B + 1
    ///
    /// B[2]: GD
    /// Selects first output gate.
    /// GD = 0, Selects gate sequence G0, G1, G2, G3... [POR]
    /// GD = 1, Selects gate sequence G1, G0, G3, G2...
    ///
    /// B[1]: SM
    /// Change scanning order of gate driver.
    /// SM = 0, Selects left and ight gates interlaced: G0, G1, G2...G299. [POR]
    /// SM = 1, Selects left and right gates separated: G0, G2, G4...G298, G1,
    /// G3...G299.
    ///
    /// B[0]: TB
    /// TB = 0, scan from G0 to G299. [POR]
    /// TB = 1, scan from G299 to G0.
    DriverOutputControl = 0x01,
    /// Data: A[7:0], B[7:0], C[7:0]
    ///
    /// Booster enable with Phase 1, Phase 2, and Phase 3 for soft start current
    /// settings.
    /// A[7:0]: Phase 1
    /// A = 0x87 [POR CFAP200200A0-154]
    /// A = 0xCE [POR CFAP200200A1-154]
    /// B[7:0]: Phase 2
    /// B = 0x86 [POR CFAP200200A0-154]
    /// B = 0xCE [POR CFAP200200A1-154]
    /// C[7:0]: Phase 3
    /// C = 0x85 [POR CFAP200200A0-154]
    /// C = 0x8D [POR CFAP200200A1-154]
    ///
    /// Set to A = 0xD7, B = 0xD6, and C = 0x9D in both the CFAP200200A0-154
    /// sample code and CFAP200200A1-154 sample code.
    BoosterSoftStartControl = 0x0c,
    /// Data: A[7:0], {[0; 7], A[8]}
    ///
    /// Sets the scanning start position of the gate driver. The valid range is
    /// from 0 to 299.
    /// A = 0. [POR]
    GateScanStartPosition = 0x0f,
    /// Data: {[0; 7], A[8]}
    ///
    /// A[0] = 0, Normal Mode [POR]
    /// A[1] = 1, Deep Sleep
    DeepSleepMode = 0x10,
    /// Data: {[0; 5], A[2:0]}
    ///
    /// A[1:0]: ID
    /// Address automatic increment/decrement setting.
    /// ID = 0b00: Y decrement, X decrement
    /// ID = 0b01: Y decrement, X increment
    /// ID = 0b10: Y increment, X decrement
    /// ID = 0b11: Y increment, X increment [POR]
    ///
    /// A[2]: AM
    /// Set the direction in which the address counter is updated automatically
    /// after data is written to the RAM.
    /// AM = 0, The address counter is updated in the X direction. [POR]
    /// AM = 1, The address counter is updated in the Y direction. 
    DataEntryModeSetting = 0x11,
    /// Resets the commands and parameters to their POR default values except
    /// 0x10, Deep Sleep Mode. RAM is uneffected by this command.
    SwReset = 0x12,
    /// Data: A[11:4], {A[3:0], [0; 4]}
    ///
    /// Write a 12-bit temperature value read from an external sensor.
    ///
    /// A temperature in Celsius can be transformed into the proper value by
    /// finding the two's complement representation of temperature multiplied
    /// by 16.
    /// A[11:0] = Temperature * 16
    /// A = 0x7FF [POR]
    ///
    /// Examples:
    /// Temp = 25 C, A = 400 = 0x190
    /// Temp = -25 C, A = -400 = 0xE70
    TemperatureSensorControl = 0x1a,
    /// Activate the Display Update Sequence. The Display Update Sequence Option
    /// is located at 0x22 (DisplayUpdateControl2). The user should not
    /// interrupt this operation to avoid corruption of panel images.
    MasterActivation = 0x20,
    /// Data: {A, [0; 2], B, C[1:0], D[1:0]}
    ///
    /// Controls the Display Update Bypass options used for Pattern Display,
    /// which is used to display the RAM content onto the display.
    ///
    /// A: Old RAM Bypass option
    /// A = 0, Disable bypass [POR]
    /// A = 1, Enable bypass
    ///
    /// B: Value to be used as new RAM for bypass
    /// B = 0 [POR]
    ///
    /// C is unknown.
    ///
    /// D[1:0]: GS
    /// Initial Update option - Source control
    /// See command 0x22 (DisplayUpdateControl2) and 0x3C
    /// (BorderWaveformControl).
    /// GS = {GSA, GSB}
    /// GS = 0b01 = {GS0, GS1} [POR]
    DisplayUpdateControl1 = 0x21,
    /// Data: A[7:0]
    ///
    /// Enables and disables the stages of the Display Update Sequence. The
    /// enabled stages are performed from bit 7 to bit 0.
    ///
    /// A[7]: CLK/OSC Enable (CLKEN = 1)
    /// A[6]: CP Enable (CPEN = 1)
    /// A[5]: Load Temperature
    /// A[4]: Load LUT
    /// A[3]: Initial Display
    /// A[2]: Display Pattern
    /// A[1]: CP Disable
    /// A[0]: CLK/OSC Disable (CLKEN = 1)
    ///
    /// Unless otherwise specified CLKEN = 0. (not specified in datasheet)
    ///
    /// If CLKEN = 1
    /// If CLS = VDDIO then enable OSC.
    /// If CLS = VSS then enable External Clock.
    ///
    /// If CLKEN = 0
    /// If CLS = VDDIO then disable OSC.
    /// If CLS = VSS then disable External Clock. (not specified in datasheet)
    ///
    /// The author of this library has no idea what CLS refers to. It seems to
    /// be a pin that does not exist.
    DisplayUpdateControl2 = 0x22,
    /// Data sent after this command is written to RAM until another command is
    /// written. Address pointer advance according to the ID setting set with
    /// command 0x11 (DataEntryModeSetting).
    WriteRam = 0x24,
    /// Data: A[7:0]
    ///
    /// Write VCOM register register.
    ///
    /// Set to 0xA8 in the CFAP200200A0-154 sample code.
    /// Set to 0x7F in the CFAP200200A1-154 sample code.
    WriteVcomRegister = 0x2c,
    /// Data: [u8; 30]
    ///
    /// Write the LUT register.
    WriteLutRegister = 0x32,
    /// Data: {0, A[6:0]}
    ///
    /// Set the number of dummy line periods in terms of TGate.
    /// A = 0x16 [POR CFAP200200A0-154]
    /// A = 0x1B [POR CFAP200200A1-154] (datasheet unclear)
    ///
    /// The author of this library does not know what TGate is.
    ///
    /// Set to 0x1A in both the CFAP200200A0-154 sample code and
    /// CFAP200200A1-154 sample code.
    SetDummyLinePeriod = 0x3a,
    /// Data: {[0; 4], A[3:0]}
    ///
    /// Based on comments from sample code, controls the timing per line.
    ///
    /// Set to 0x08 in the CFAP200200A0-154 sample code.
    /// Not set in the CFAP200200A1-154 sample code.
    SetGateLineWidth = 0x3b,
    /// Data: {A, B, C[1:0], [0; 2], D[1:0]}
    ///
    /// Selects the border waveform for the VBD.
    ///
    /// A: Follow source at initial Update Display
    /// A = 0 [POR]
    /// A = 1, Follow source at initial Update Display for VBD. Settings B, C,
    /// and D are overridden at Initial Display stage.
    ///
    /// B: Select GS Transition/Fix level for VBD
    /// B = 0, Select GS Transition D[3:2] for VBD. May be related to command
    /// 0x20 (DisplayUpdateControl1).
    /// B = 1, Select fix level setting C[1:0] for VBD. [POR]
    ///
    /// C[1:0]: Fix level setting for VBD
    /// C = 0b00, VBD level is VSS.
    /// C = 0b01, VBD level is VSH.
    /// C = 0b10, VBD level is VSL.
    /// C = 0b11, VBD level is HiZ. [POR]
    ///
    /// D[1:0]: GS transition for VBD
    /// See command 0x21 (DisplayUpdateControl1) and 0x22
    /// (DisplayUpdateControl2).
    /// GS = {GSC, GSD}
    /// GS = 0b01 = {GS0, GS1} [POR]
    BorderWaveformControl = 0x3c,
    /// Data: {[0; 3], A[4:0]}, {[0; 3], B[4:0]}
    ///
    /// Specify the start and end positions of the window address in the X
    /// direction.
    ///
    /// A[4:0]: XSA, X start
    /// XSA = 0x00 [POR]
    ///
    /// B[4:0]: XEA, X end
    /// XEA = 0x18 [POR]
    SetRamXAddressStartEndPosition = 0x44,
    /// Data: A[7:0], {[0; 7], A[8]}, B[7:0], {[0; 7], B[8]}
    ///
    /// Specify the start and end positions of the window address in the Y
    /// direction.
    ///
    /// A[8:0]: YSA, Y start
    /// YSA = 0x000 [POR]
    ///
    /// B[8:0]: YEA, Y end
    /// YEA = 0x12B [POR]
    SetRamYAddressStartEndPosition = 0x45,
    /// Data: {[0; 3], A[4:0]}
    ///
    /// Initial setting for the X address counter.
    ///
    /// A[4:0]: XAD
    /// XAD = 0x00 [POR]
    SetRamXAddressCounter = 0x4e,
    /// Data: A[7:0], {[0; 7], A[8]}
    ///
    /// Initial setting for the X address counter.
    ///
    /// A[8:0]: YAD
    /// YAD = 0x000 [POR]
    SetRamYAddressCounter = 0x4f,
    /// This command is an empty command. It does not have any effect on the
    /// display module. It can be used to terminate command 0x24 (WriteRam).
    Nop = 0xff,
}

pub const SOFT_START_CFAP200200A0_154: [u8; 3] = [0xd7, 0xd6, 0x9d];
pub const SOFT_START_CFAP200200A1_154: [u8; 3] = [0xd7, 0xd6, 0x9d];

pub const VCOM_CFAP200200A0_154: u8 = 0xA8;
pub const VCOM_CFAP200200A1_154: u8 = 0x7F;

pub const DUMMY_LINE_CFAP200200A0_154: u8 = 0x1A;
pub const DUMMY_LINE_CFAP200200A1_154: u8 = 0x1A;

pub const GATE_LINE_CFAP200200A0_154: u8 = 0x08;
pub const GATE_LINE_CFAP200200A1_154: u8 = 0x08;

pub const LUT_FULL_CFAP200200A0_154: [u8; 30] = [
    0x02,
    0x02,
    0x01,
    0x11,
    0x12,
    0x12,
    0x22,
    0x22,
    0x66,
    0x69,
    0x69,
    0x59,
    0x58,
    0x99,
    0x99,
    0x88,
    0x00,
    0x00,
    0x00,
    0x00,
    0xF8,
    0xB4,
    0x13,
    0x51,
    0x35,
    0x51,
    0x51,
    0x19,
    0x01,
    0x00,
];

pub const LUT_FULL_CFAP200200A1_154: [u8; 30] = [
    0x66,
    0x66,
    0x44,
    0x66,
    0xAA,
    0x11,
    0x80,
    0x08,
    0x11,
    0x18,
    0x81,
    0x18,
    0x11,
    0x88,
    0x11,
    0x88,
    0x11,
    0x88,
    0x00,
    0x00,
    0xFF,
    0xFF,
    0xFF,
    0xFF,
    0x5F,
    0xAF,
    0xFF,
    0xFF,
    0x2F,
    0x00,
];

pub const LUT_PART_CFAP200200A0_154: [u8; 30] = [
    0x10,
    0x18,
    0x18,
    0x08,
    0x18,
    0x18,
    0x08,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x13,
    0x14,
    0x44,
    0x12,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
];

pub const LUT_PART_CFAP200200A1_154: [u8; 30] = [
    0x10,
    0x18,
    0x18,
    0x28,
    0x18,
    0x18,
    0x18,
    0x18,
    0x08,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x13,
    0x11,
    0x22,
    0x63,
    0x11,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
];

#[derive(Debug)]
pub enum ScreenError<ERR> {
    BoundsError,
    LengthError,
    SpiError(ERR)
}

impl<ERR> From<ERR> for ScreenError<ERR> {
    fn from(err:ERR) -> Self {
        ScreenError::SpiError(err)
    }
}

impl<ERR> Clone for ScreenError<ERR> where ERR: Clone {
    fn clone(&self) -> Self {
        match self {
            ScreenError::BoundsError => ScreenError::BoundsError,
            ScreenError::LengthError => ScreenError::LengthError,
            ScreenError::SpiError(err) => ScreenError::SpiError(err.clone()),
        }
    }
}

impl<ERR> Copy for ScreenError<ERR> where ERR: Copy {}

pub struct Screen<SPI, DC, CS, BUSY, RST, ERR>
where
    SPI: FullDuplex<u8, Error = ERR>,
    DC: OutputPin,
    CS: OutputPin,
    BUSY: InputPin,
    RST: OutputPin,
{
    /// up to 4 MHz, MSB first, SPI mode 0
    serial: SPI,
    dc: DC,
    cs: CS,
    busy: BUSY,
    reset: RST,

    // TODO: const generics
    x_size: u16,
    y_size: u16,
    lut_full: [u8; 30],
    lut_part: [u8; 30],
}

impl<SPI, DC, CS, BUSY, RST, ERR> Screen<SPI, DC, CS, BUSY, RST, ERR>
where
    SPI: FullDuplex<u8, Error = ERR>,
    DC: OutputPin,
    CS: OutputPin,
    BUSY: InputPin,
    RST: OutputPin,
{
    fn new<DELAY>(
        serial: SPI,
        dc: DC,
        cs: CS,
        busy: BUSY,
        reset: RST,
        builder: ScreenBuilder,
        delay: &mut DELAY,
    ) -> Result<Screen<SPI, DC, CS, BUSY, RST, ERR>, ERR>
    where
        DELAY: DelayMs<u16>,
    {
        let mut screen = Screen {
            serial,
            dc,
            cs,
            busy,
            reset,
            x_size: builder.x_size,
            y_size: builder.y_size,
            lut_full: builder.lut_full,
            lut_part: builder.lut_part
        };

        screen.reset.set_low();
        screen.cs.set_high();
        screen.dc.set_high();

        // TODO: determine actual minimums for reset timing
        delay.delay_ms(10);
        screen.reset.set_high();
        delay.delay_ms(10);

        // Panel configuration, Gate selection
        screen.write_cmd_string(
            Command::DriverOutputControl,
            &[builder.x_size as u8, ((builder.x_size >> 8) & 0x01) as u8, 0x00]
        )?;
        screen.write_cmd_string(
            Command::BoosterSoftStartControl,
            &builder.soft_start
        )?;
        // VCOM setting
        screen.write_cmd_string(
            Command::WriteVcomRegister,
            &[builder.vcom]
        )?;
        //dummy line per gate
        screen.write_cmd_string(
            Command::SetDummyLinePeriod,
            &[builder.dummy_line]
        )?;
        // Gate time setting
        screen.write_cmd_string(
            Command::SetGateLineWidth,
            &[builder.gate_line]
        )?;
        // X increase, Y decrease
        screen.write_cmd_string(
            Command::DataEntryModeSetting,
            &[builder.entry_mode as u8]
        )?;

        Ok(screen)
    }

    pub fn show_full_screen_image(&mut self, image: &[u8]) -> Result<(), ScreenError<ERR>> {
        let x_size = ((self.x_size + 7) >> 3) as u8;
        let y_size = self.y_size;

        if x_size as usize * y_size as usize != image.len() {
            return Err(ScreenError::LengthError);
        }

        self.load_full_update_lut()?;
        self.power_on()?;

        self.set_display_area(
            0, x_size - 1,
            y_size - 1, 0
        )?;

        self.load_image(image)?;
        self.update_full()?;

        self.power_off()?;

        Ok(())
    }

    /// `x_start` and `x_size` are in bytes. `y_start` and `y_size` are in pixels.
    /// `y_start` must be greater than or equal to `y_size`.
    pub fn load_partial_image(
        &mut self,
        x_start: u8, x_size: u8,
        y_start: u16, y_size: u16,
        image: &[u8],
    ) -> Result<(), ScreenError<ERR>> {
        if x_size as usize * y_size as usize != image.len() {
            return Err(ScreenError::LengthError);
        }

        self.set_display_area(
            x_start, x_start + x_size - 1,
            y_start, y_start - (y_size - 1),
        )?;
        self.load_image(image)?;

        Ok(())
    }

    pub fn load_full_update_lut(&mut self) -> Result<(), ERR> {
        let lut_full_update = self.lut_full;

        self.write_cmd_string(Command::WriteLutRegister, &lut_full_update)
    }

    pub fn load_partial_update_lut(&mut self) -> Result<(), ERR> {
        let lut_partial_update = self.lut_part;

        self.write_cmd_string(Command::WriteLutRegister, &lut_partial_update)
    }

    pub fn power_on(&mut self) -> Result<(), ERR> {
        self.write_cmd_string(Command::DisplayUpdateControl2, &[0xc0])?;
        self.write_cmd(Command::MasterActivation)?;

        while self.busy.is_high() {}

        Ok(())
    }

    pub fn power_off(&mut self) -> Result<(), ERR> {
        self.write_cmd_string(Command::DisplayUpdateControl2, &[0xc3])?;
        self.write_cmd(Command::MasterActivation)
    }

    pub fn update_full(&mut self) -> Result<(), ERR> {
        //    C    7
        // 1100 0111
        // |||| ||||-- CLK/OSC DISABLE  (0x01)
        // |||| |||--- CP DISABLE       (0x02)
        // |||| ||---- DISPLAY_PATTERN  (0x04) <<
        // |||| |----- INITIAL DISPLAY  (0x08)
        // ||||------- LOAD LUT         (0x10)
        // |||-------- LOAD TEMPERATURE (0x20)
        // ||--------- CP ENABLE        (0x40)
        // |---------- CLK/OSC ENABLE   (0x80)
        self.write_cmd_string(Command::DisplayUpdateControl2, &[0xc7])?;
        self.write_cmd(Command::MasterActivation)?;
        self.write_cmd(Command::Nop)
    }

    pub fn update_partial(&mut self) -> Result<(), ERR> {
        //    0    4
        // 0000 0100
        // |||| ||||-- CLK/OSC DISABLE  (0x01)
        // |||| |||--- CP DISABLE       (0x02)
        // |||| ||---- DISPLAY_PATTERN  (0x04)  <<
        // |||| |----- INITIAL DISPLAY  (0x08)
        // ||||------- LOAD LUT         (0x10)
        // |||-------- LOAD TEMPERATURE (0x20)
        // ||--------- CP ENABLE        (0x40)
        // |---------- CLK/OSC ENABLE   (0x80)
        self.write_cmd_string(Command::DisplayUpdateControl2, &[0x04])?;
        self.write_cmd(Command::MasterActivation)?;
        self.write_cmd(Command::Nop)
    }

    pub fn load_image(&mut self, image: &[u8]) -> Result<(), ERR> {
        while self.busy.is_high() {}

        self.cs.set_low();

        self.dc.set_low();
        block!(self.serial.send(Command::WriteRam as u8))?;
        let _ = block!(self.serial.read())?;

        self.dc.set_high();
        for d in image {
            block!(self.serial.send(*d))?;
            let _ = block!(self.serial.read())?;
        }

        self.cs.set_high();

        Ok(())
    }

    /// `x_start` and `x_end` are in bytes. `y_start` and `y_end` are in pixels.
    pub fn set_display_area(&mut self, x_start: u8, x_end: u8, y_start: u16, y_end: u16) -> Result<(), ScreenError<ERR>> {
        let x_size = width_pixels_to_bytes(self.x_size);
        if x_start > x_size || x_end > x_size || y_start > self.y_size || y_end > self.y_size {
            return Err(ScreenError::BoundsError);
        }

        // set x region
        self.write_cmd_string(
            Command::SetRamXAddressStartEndPosition,
            &[x_start, x_end]
        )?;
        // set y region
        self.write_cmd_string(
            Command::SetRamYAddressStartEndPosition,
            &[y_start as u8, (y_start >> 8) as u8, y_end as u8, (y_end >> 8) as u8]
        )?;
        // set x origin
        self.write_cmd_string(
            Command::SetRamXAddressCounter,
            &[x_start]
        )?;
        // set y origin
        self.write_cmd_string(
            Command::SetRamYAddressCounter,
            &[y_start as u8, (y_start >> 8) as u8]
        )?;

        Ok(())
    }

    pub fn write_cmd(&mut self, cmd: Command) -> Result<(), ERR> {
        self.cs.set_low();

        self.dc.set_low();
        block!(self.serial.send(cmd as u8))?;
        let _ = block!(self.serial.read())?;

        self.cs.set_high();

        Ok(())
    }

    pub fn write_data(&mut self, data: u8) -> Result<(), ERR> {
        self.cs.set_low();
    
        self.dc.set_high();
        block!(self.serial.send(data))?;
        let _ = block!(self.serial.read())?;
    
        self.cs.set_high();
    
        Ok(())
    }

    pub fn write_cmd_string(&mut self, cmd: Command, data: &[u8]) -> Result<(), ERR> {
        self.cs.set_low();

        self.dc.set_low();
        block!(self.serial.send(cmd as u8))?;
        let _ = block!(self.serial.read())?;

        self.dc.set_high();
        for d in data {
            block!(self.serial.send(*d))?;
            let _ = block!(self.serial.read())?;
        }

        self.cs.set_high();

        Ok(())
    }
}
