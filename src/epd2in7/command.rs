//! SPI Commands for the GDEY027T91 2.7" E-Ink Display (SSD1680-compatible)

use crate::traits;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum Command {
    DriverOutputControl = 0x01,
    GateScanStartPosition = 0x0F,
    DeepSleepMode = 0x10,
    DataEntryModeSetting = 0x11,
    SwReset = 0x12,
    TemperatureSensorControl = 0x18,
    TemperatureSensorWrite = 0x1A,
    MasterActivation = 0x20,
    DisplayUpdateControl1 = 0x21,
    DisplayUpdateControl2 = 0x22,
    WriteRam = 0x24,
    WriteRamRed = 0x26,
    BorderWaveformControl = 0x3C,
    SetRamXAddressStartEndPosition = 0x44,
    SetRamYAddressStartEndPosition = 0x45,
    SetRamXAddressCounter = 0x4E,
    SetRamYAddressCounter = 0x4F,
}

impl traits::Command for Command {
    fn address(self) -> u8 {
        self as u8
    }
}
