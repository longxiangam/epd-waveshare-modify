//! A Driver for the GDEY027T91 2.7" E-Ink Display via SPI
//!
//! Based on the Good Display GDEY027T91 panel with SSD1680-compatible controller.
//! Supports full refresh, fast refresh, and partial refresh modes.

use embedded_hal::{
    {delay::*, spi::SpiDevice},
    digital::*,
};

use crate::buffer_len;
use crate::color::Color;
use crate::interface::DisplayInterface;
use crate::traits::{InternalWiAdditions, QuickRefresh, RefreshLut, WaveshareDisplay};

pub(crate) mod command;
use self::command::Command;

#[cfg(feature = "graphics")]
mod graphics;
#[cfg(feature = "graphics")]
pub use self::graphics::Display2in7;

/// Width of the display
pub const WIDTH: u32 = 176;
/// Height of the display
pub const HEIGHT: u32 = 264;
/// Default Background Color
pub const DEFAULT_BACKGROUND_COLOR: Color = Color::White;

const IS_BUSY_LOW: bool = false;

/// Epd2in7 driver for GDEY027T91
pub struct Epd2in7<SPI, BUSY, DC, RST, DELAY> {
    interface: DisplayInterface<SPI, BUSY, DC, RST, DELAY>,
    background_color: Color,
    refresh: RefreshLut,
}

impl<SPI, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, BUSY, DC, RST, DELAY>
    for Epd2in7<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.reset(delay, 10);
        self.wait_until_idle();

        self.command(spi, Command::SwReset)?;
        self.wait_until_idle();

        if self.refresh == RefreshLut::Quick {
            self.init_fast_refresh(spi)?;
        }

        self.wait_until_idle();
        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, BUSY, DC, RST, DELAY>
    for Epd2in7<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    type DisplayColor = Color;

    fn new(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
    ) -> Result<Self, SPI::Error> {
        let mut epd = Epd2in7 {
            interface: DisplayInterface::new(busy, dc, rst),
            background_color: DEFAULT_BACKGROUND_COLOR,
            refresh: RefreshLut::Full,
        };

        epd.init(spi, delay)?;
        Ok(epd)
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn sleep(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle();
        self.cmd_with_data(spi, Command::DeepSleepMode, &[0x01])?;
        Ok(())
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        _delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        assert!(buffer.len() == buffer_len(WIDTH as usize, HEIGHT as usize));

        self.command(spi, Command::WriteRam)?;
        self.interface.data(spi, buffer)?;

        if self.refresh == RefreshLut::Full {
            self.display_update_sequence(spi, 0xF7)?;
        }
        Ok(())
    }

    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        assert!((width * height / 8) as usize == buffer.len());

        self.cmd_with_data(spi, Command::BorderWaveformControl, &[0x80])?;

        self.set_ram_area(spi, x, y, x + width - 1, y + height - 1)?;
        self.set_ram_address_counters(spi, x, y)?;

        self.command(spi, Command::WriteRam)?;
        self.interface.data(spi, buffer)?;

        self.display_update_sequence(spi, 0xFF)?;
        Ok(())
    }

    fn display_frame(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        let seq = match self.refresh {
            RefreshLut::Full => 0xF7,
            RefreshLut::Quick => 0xC7,
        };
        self.display_update_sequence(spi, seq)
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)
    }

    fn clear_frame(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        let color = self.background_color.get_byte_value();

        self.command(spi, Command::WriteRam)?;
        self.interface
            .data_x_times(spi, color, buffer_len(WIDTH as usize, HEIGHT as usize) as u32)?;

        self.display_update_sequence(spi, 0xF7)?;
        Ok(())
    }

    fn set_background_color(&mut self, background_color: Color) {
        self.background_color = background_color;
    }

    fn background_color(&self) -> &Color {
        &self.background_color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn set_lut(
        &mut self,
        _spi: &mut SPI,
        refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        if let Some(refresh) = refresh_rate {
            self.refresh = refresh;
        }
        Ok(())
    }

    fn is_busy(&mut self) -> bool {
        self.interface.is_busy(IS_BUSY_LOW)
    }
}

impl<SPI, BUSY, DC, RST, DELAY> QuickRefresh<SPI, BUSY, DC, RST, DELAY>
    for Epd2in7<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn update_old_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        _delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        assert!(buffer.len() == buffer_len(WIDTH as usize, HEIGHT as usize));

        self.command(spi, Command::WriteRamRed)?;
        self.interface.data(spi, buffer)?;
        Ok(())
    }

    fn update_new_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        _delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        assert!(buffer.len() == buffer_len(WIDTH as usize, HEIGHT as usize));

        self.command(spi, Command::WriteRam)?;
        self.interface.data(spi, buffer)?;
        Ok(())
    }

    fn display_new_frame(&mut self, spi: &mut SPI, _delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.display_update_sequence(spi, 0xC7)
    }

    fn update_and_display_new_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_new_frame(spi, buffer, delay)?;
        self.display_new_frame(spi, delay)
    }

    fn update_partial_old_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        self.set_ram_area(spi, x, y, x + width - 1, y + height - 1)?;
        self.set_ram_address_counters(spi, x, y)?;

        self.command(spi, Command::WriteRamRed)?;
        self.interface.data(spi, buffer)?;
        Ok(())
    }

    fn update_partial_new_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        self.set_ram_area(spi, x, y, x + width - 1, y + height - 1)?;
        self.set_ram_address_counters(spi, x, y)?;

        self.command(spi, Command::WriteRam)?;
        self.interface.data(spi, buffer)?;
        Ok(())
    }

    fn clear_partial_frame(
        &mut self,
        spi: &mut SPI,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        let color = self.background_color.get_byte_value();

        self.set_ram_area(spi, x, y, x + width - 1, y + height - 1)?;
        self.set_ram_address_counters(spi, x, y)?;

        self.command(spi, Command::WriteRam)?;
        self.interface
            .data_x_times(spi, color, (width / 8 * height) as u32)?;
        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> Epd2in7<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    /// Sets the refresh mode. When changing mode, the screen will be
    /// re-initialized accordingly.
    pub fn set_refresh(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        refresh: RefreshLut,
    ) -> Result<(), SPI::Error> {
        if self.refresh != refresh {
            self.refresh = refresh;
            self.init(spi, delay)?;
        }
        Ok(())
    }

    fn init_fast_refresh(&mut self, spi: &mut SPI) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::TemperatureSensorControl, &[0x80])?;

        self.cmd_with_data(spi, Command::DisplayUpdateControl2, &[0xB1])?;
        self.command(spi, Command::MasterActivation)?;
        self.wait_until_idle();

        self.cmd_with_data(spi, Command::TemperatureSensorWrite, &[0x64, 0x00])?;

        self.cmd_with_data(spi, Command::DisplayUpdateControl2, &[0x91])?;
        self.command(spi, Command::MasterActivation)?;
        self.wait_until_idle();

        Ok(())
    }

    fn display_update_sequence(
        &mut self,
        spi: &mut SPI,
        seq: u8,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(spi, Command::DisplayUpdateControl2, &[seq])?;
        self.command(spi, Command::MasterActivation)?;
        self.wait_until_idle();
        Ok(())
    }

    fn set_ram_area(
        &mut self,
        spi: &mut SPI,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) -> Result<(), SPI::Error> {
        self.cmd_with_data(
            spi,
            Command::SetRamXAddressStartEndPosition,
            &[(start_x >> 3) as u8, (end_x >> 3) as u8],
        )?;

        self.cmd_with_data(
            spi,
            Command::SetRamYAddressStartEndPosition,
            &[
                start_y as u8,
                (start_y >> 8) as u8,
                end_y as u8,
                (end_y >> 8) as u8,
            ],
        )
    }

    fn set_ram_address_counters(
        &mut self,
        spi: &mut SPI,
        x: u32,
        y: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle();
        self.cmd_with_data(spi, Command::SetRamXAddressCounter, &[(x >> 3) as u8])?;
        self.cmd_with_data(
            spi,
            Command::SetRamYAddressCounter,
            &[y as u8, (y >> 8) as u8],
        )
    }

    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn cmd_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }

    fn wait_until_idle(&mut self) {
        let _ = self.interface.wait_until_idle(IS_BUSY_LOW);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epd_size() {
        assert_eq!(WIDTH, 176);
        assert_eq!(HEIGHT, 264);
        assert_eq!(DEFAULT_BACKGROUND_COLOR, Color::White);
        assert_eq!(buffer_len(WIDTH as usize, HEIGHT as usize), 5808);
    }
}
