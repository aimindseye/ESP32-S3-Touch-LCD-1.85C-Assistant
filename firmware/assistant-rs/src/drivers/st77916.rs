#![allow(dead_code)]

use super::tca9554::Tca9554;
use crate::board;
use embedded_hal::i2c::I2c;
use esp_hal::delay::Delay;

#[derive(Debug, Clone, Copy)]
pub struct InitCmd {
    pub cmd: u8,
    pub data: &'static [u8],
    pub delay_ms: u16,
}

impl InitCmd {
    pub const fn new(cmd: u8, data: &'static [u8], delay_ms: u16) -> Self {
        Self { cmd, data, delay_ms }
    }
}

#[derive(Debug)]
pub enum Error<E> {
    Bus(E),
    Pin,
    Transport,
}

pub trait St77916Transport {
    fn write_param(&mut self, cmd: u8, data: &[u8]) -> Result<(), ()>;
    fn write_color(&mut self, data: &[u8]) -> Result<(), ()>;
}

pub struct St77916;

impl St77916 {
    pub const INIT_TABLE: &'static [InitCmd] = &[
        InitCmd::new(0xF0, &[0x28], 0),
        InitCmd::new(0xF2, &[0x28], 0),
        InitCmd::new(0x73, &[0xF0], 0),
        InitCmd::new(0x7C, &[0xD1], 0),
        InitCmd::new(0x83, &[0xE0], 0),
        InitCmd::new(0x84, &[0x61], 0),
        InitCmd::new(0xF2, &[0x82], 0),
        InitCmd::new(0xF0, &[0x00], 0),
        InitCmd::new(0xF0, &[0x01], 0),
        InitCmd::new(0xF1, &[0x01], 0),
        InitCmd::new(0xB0, &[0x56], 0),
        InitCmd::new(0xB1, &[0x4D], 0),
        InitCmd::new(0xB2, &[0x24], 0),
        InitCmd::new(0xB4, &[0x87], 0),
        InitCmd::new(0xB5, &[0x44], 0),
        InitCmd::new(0xB6, &[0x8B], 0),
        InitCmd::new(0xB7, &[0x40], 0),
        InitCmd::new(0xB8, &[0x86], 0),
        InitCmd::new(0xBA, &[0x00], 0),
        InitCmd::new(0xBB, &[0x08], 0),
        InitCmd::new(0xBC, &[0x08], 0),
        InitCmd::new(0xBD, &[0x00], 0),
        InitCmd::new(0xC0, &[0x80], 0),
        InitCmd::new(0xC1, &[0x10], 0),
        InitCmd::new(0xC2, &[0x37], 0),
        InitCmd::new(0xC3, &[0x80], 0),
        InitCmd::new(0xC4, &[0x10], 0),
        InitCmd::new(0xC5, &[0x37], 0),
        InitCmd::new(0xC6, &[0xA9], 0),
        InitCmd::new(0xC7, &[0x41], 0),
        InitCmd::new(0xC8, &[0x01], 0),
        InitCmd::new(0xC9, &[0xA9], 0),
        InitCmd::new(0xCA, &[0x41], 0),
        InitCmd::new(0xCB, &[0x01], 0),
        InitCmd::new(0xD0, &[0x91], 0),
        InitCmd::new(0xD1, &[0x68], 0),
        InitCmd::new(0xD2, &[0x68], 0),
        InitCmd::new(0xF5, &[0x00, 0xA5], 0),
        InitCmd::new(0xDD, &[0x4F], 0),
        InitCmd::new(0xDE, &[0x4F], 0),
        InitCmd::new(0xF1, &[0x10], 0),
        InitCmd::new(0xF0, &[0x00], 0),
        InitCmd::new(0xF0, &[0x02], 0),
        InitCmd::new(
            0xE0,
            &[
                0xF0, 0x0A, 0x10, 0x09, 0x09, 0x36, 0x35, 0x33, 0x4A, 0x29, 0x15, 0x15, 0x2E,
                0x34,
            ],
            0,
        ),
        InitCmd::new(
            0xE1,
            &[
                0xF0, 0x0A, 0x0F, 0x08, 0x08, 0x05, 0x34, 0x33, 0x4A, 0x39, 0x15, 0x15, 0x2D,
                0x33,
            ],
            0,
        ),
        InitCmd::new(0xF0, &[0x10], 0),
        InitCmd::new(0xF3, &[0x10], 0),
        InitCmd::new(0xE0, &[0x07], 0),
        InitCmd::new(0xE1, &[0x00], 0),
        InitCmd::new(0xE2, &[0x00], 0),
        InitCmd::new(0xE3, &[0x00], 0),
        InitCmd::new(0xE4, &[0xE0], 0),
        InitCmd::new(0xE5, &[0x06], 0),
        InitCmd::new(0xE6, &[0x21], 0),
        InitCmd::new(0xE7, &[0x01], 0),
        InitCmd::new(0xE8, &[0x05], 0),
        InitCmd::new(0xE9, &[0x02], 0),
        InitCmd::new(0xEA, &[0xDA], 0),
        InitCmd::new(0xEB, &[0x00], 0),
        InitCmd::new(0xEC, &[0x00], 0),
        InitCmd::new(0xED, &[0x0F], 0),
        InitCmd::new(0xEE, &[0x00], 0),
        InitCmd::new(0xEF, &[0x00], 0),
        InitCmd::new(0xF8, &[0x00], 0),
        InitCmd::new(0xF9, &[0x00], 0),
        InitCmd::new(0xFA, &[0x00], 0),
        InitCmd::new(0xFB, &[0x00], 0),
        InitCmd::new(0xFC, &[0x00], 0),
        InitCmd::new(0xFD, &[0x00], 0),
        InitCmd::new(0xFE, &[0x00], 0),
        InitCmd::new(0xFF, &[0x00], 0),
        InitCmd::new(0x60, &[0x40], 0),
        InitCmd::new(0x61, &[0x04], 0),
        InitCmd::new(0x62, &[0x00], 0),
        InitCmd::new(0x63, &[0x42], 0),
        InitCmd::new(0x64, &[0xD9], 0),
        InitCmd::new(0x65, &[0x00], 0),
        InitCmd::new(0x66, &[0x00], 0),
        InitCmd::new(0x67, &[0x00], 0),
        InitCmd::new(0x68, &[0x00], 0),
        InitCmd::new(0x69, &[0x00], 0),
        InitCmd::new(0x6A, &[0x00], 0),
        InitCmd::new(0x6B, &[0x00], 0),
        InitCmd::new(0x70, &[0x40], 0),
        InitCmd::new(0x71, &[0x03], 0),
        InitCmd::new(0x72, &[0x00], 0),
        InitCmd::new(0x73, &[0x42], 0),
        InitCmd::new(0x74, &[0xD8], 0),
        InitCmd::new(0x75, &[0x00], 0),
        InitCmd::new(0x76, &[0x00], 0),
        InitCmd::new(0x77, &[0x00], 0),
        InitCmd::new(0x78, &[0x00], 0),
        InitCmd::new(0x79, &[0x00], 0),
        InitCmd::new(0x7A, &[0x00], 0),
        InitCmd::new(0x7B, &[0x00], 0),
        InitCmd::new(0x80, &[0x48], 0),
        InitCmd::new(0x81, &[0x00], 0),
        InitCmd::new(0x82, &[0x06], 0),
        InitCmd::new(0x83, &[0x02], 0),
        InitCmd::new(0x84, &[0xD6], 0),
        InitCmd::new(0x85, &[0x04], 0),
        InitCmd::new(0x86, &[0x00], 0),
        InitCmd::new(0x87, &[0x00], 0),
        InitCmd::new(0x88, &[0x48], 0),
        InitCmd::new(0x89, &[0x00], 0),
        InitCmd::new(0x8A, &[0x08], 0),
        InitCmd::new(0x8B, &[0x02], 0),
        InitCmd::new(0x8C, &[0xD8], 0),
        InitCmd::new(0x8D, &[0x04], 0),
        InitCmd::new(0x8E, &[0x00], 0),
        InitCmd::new(0x8F, &[0x00], 0),
        InitCmd::new(0x90, &[0x48], 0),
        InitCmd::new(0x91, &[0x00], 0),
        InitCmd::new(0x92, &[0x0A], 0),
        InitCmd::new(0x93, &[0x02], 0),
        InitCmd::new(0x94, &[0xDA], 0),
        InitCmd::new(0x95, &[0x04], 0),
        InitCmd::new(0x96, &[0x00], 0),
        InitCmd::new(0x97, &[0x00], 0),
        InitCmd::new(0x98, &[0x48], 0),
        InitCmd::new(0x99, &[0x00], 0),
        InitCmd::new(0x9A, &[0x0C], 0),
        InitCmd::new(0x9B, &[0x02], 0),
        InitCmd::new(0x9C, &[0xDC], 0),
        InitCmd::new(0x9D, &[0x04], 0),
        InitCmd::new(0x9E, &[0x00], 0),
        InitCmd::new(0x9F, &[0x00], 0),
        InitCmd::new(0xA0, &[0x48], 0),
        InitCmd::new(0xA1, &[0x00], 0),
        InitCmd::new(0xA2, &[0x05], 0),
        InitCmd::new(0xA3, &[0x02], 0),
        InitCmd::new(0xA4, &[0xD5], 0),
        InitCmd::new(0xA5, &[0x04], 0),
        InitCmd::new(0xA6, &[0x00], 0),
        InitCmd::new(0xA7, &[0x00], 0),
        InitCmd::new(0xA8, &[0x48], 0),
        InitCmd::new(0xA9, &[0x00], 0),
        InitCmd::new(0xAA, &[0x07], 0),
        InitCmd::new(0xAB, &[0x02], 0),
        InitCmd::new(0xAC, &[0xD7], 0),
        InitCmd::new(0xAD, &[0x04], 0),
        InitCmd::new(0xAE, &[0x00], 0),
        InitCmd::new(0xAF, &[0x00], 0),
        InitCmd::new(0xB0, &[0x48], 0),
        InitCmd::new(0xB1, &[0x00], 0),
        InitCmd::new(0xB2, &[0x09], 0),
        InitCmd::new(0xB3, &[0x02], 0),
        InitCmd::new(0xB4, &[0xD9], 0),
        InitCmd::new(0xB5, &[0x04], 0),
        InitCmd::new(0xB6, &[0x00], 0),
        InitCmd::new(0xB7, &[0x00], 0),
        InitCmd::new(0xB8, &[0x48], 0),
        InitCmd::new(0xB9, &[0x00], 0),
        InitCmd::new(0xBA, &[0x0B], 0),
        InitCmd::new(0xBB, &[0x02], 0),
        InitCmd::new(0xBC, &[0xDB], 0),
        InitCmd::new(0xBD, &[0x04], 0),
        InitCmd::new(0xBE, &[0x00], 0),
        InitCmd::new(0xBF, &[0x00], 0),
        InitCmd::new(0xC0, &[0x10], 0),
        InitCmd::new(0xC1, &[0x47], 0),
        InitCmd::new(0xC2, &[0x56], 0),
        InitCmd::new(0xC3, &[0x65], 0),
        InitCmd::new(0xC4, &[0x74], 0),
        InitCmd::new(0xC5, &[0x88], 0),
        InitCmd::new(0xC6, &[0x99], 0),
        InitCmd::new(0xC7, &[0x01], 0),
        InitCmd::new(0xC8, &[0xBB], 0),
        InitCmd::new(0xC9, &[0xAA], 0),
        InitCmd::new(0xD0, &[0x10], 0),
        InitCmd::new(0xD1, &[0x47], 0),
        InitCmd::new(0xD2, &[0x56], 0),
        InitCmd::new(0xD3, &[0x65], 0),
        InitCmd::new(0xD4, &[0x74], 0),
        InitCmd::new(0xD5, &[0x88], 0),
        InitCmd::new(0xD6, &[0x99], 0),
        InitCmd::new(0xD7, &[0x01], 0),
        InitCmd::new(0xD8, &[0xBB], 0),
        InitCmd::new(0xD9, &[0xAA], 0),
        InitCmd::new(0xF3, &[0x01], 0),
        InitCmd::new(0xF0, &[0x00], 0),
        InitCmd::new(0x21, &[0x00], 0),
        InitCmd::new(0x11, &[0x00], 120),
        InitCmd::new(0x29, &[0x00], 0),
    ];

    pub const fn new() -> Self {
        Self
    }

    pub fn reset_via_exio<I2C>(
        &self,
        exio: &mut Tca9554,
        i2c: &mut I2C,
        addr: u8,
        delay: &mut Delay,
    ) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        exio.write_pin(i2c, addr, board::EXIO_LCD_RST, false)
            .map_err(map_exio)?;
        delay.delay_millis(10);
        exio.write_pin(i2c, addr, board::EXIO_LCD_RST, true)
            .map_err(map_exio)?;
        delay.delay_millis(50);
        Ok(())
    }

    pub fn init<T>(&self, transport: &mut T, delay: &mut Delay) -> Result<(), Error<()>>
    where
        T: St77916Transport,
    {
        for item in Self::INIT_TABLE {
            transport
                .write_param(item.cmd, item.data)
                .map_err(|_| Error::Transport)?;
            if item.delay_ms != 0 {
                delay.delay_millis(item.delay_ms as u32);
            }
        }
        Ok(())
    }

    pub fn set_window<T>(
        &self,
        transport: &mut T,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
    ) -> Result<(), Error<()>>
    where
        T: St77916Transport,
    {
        let col = [(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8];
        let row = [(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8];

        transport.write_param(0x2A, &col).map_err(|_| Error::Transport)?;
        transport.write_param(0x2B, &row).map_err(|_| Error::Transport)?;
        Ok(())
    }

    pub fn flush_rgb565<T>(
        &self,
        transport: &mut T,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        pixels: &[u8],
    ) -> Result<(), Error<()>>
    where
        T: St77916Transport,
    {
        self.set_window(transport, x0, y0, x1, y1)?;
        transport.write_color(pixels).map_err(|_| Error::Transport)
    }

    #[inline]
    pub const fn pack_qspi_param_word(cmd: u8) -> u32 {
        ((board::LCD_OPCODE_WRITE_CMD as u32) << 24) | ((cmd as u32) << 8)
    }

    #[inline]
    pub const fn pack_qspi_color_word(cmd: u8) -> u32 {
        ((board::LCD_OPCODE_WRITE_COLOR as u32) << 24) | ((cmd as u32) << 8)
    }
}

fn map_exio<E>(err: super::tca9554::Error<E>) -> Error<E> {
    match err {
        super::tca9554::Error::Bus(e) => Error::Bus(e),
        super::tca9554::Error::InvalidPin => Error::Pin,
    }
}

#[cfg(feature = "qspi-display")]
pub mod esp_hal_qspi {
    use super::*;
    use esp_hal::{
        gpio::interconnect::PeripheralOutput,
        spi::{
            master::{Address, Command, Config, ConfigError, DataMode, Spi},
            Mode,
        },
        time::Rate,
        Blocking,
    };

    pub struct QspiTransport<'d> {
        spi: Spi<'d, Blocking>,
    }

    impl<'d> QspiTransport<'d> {
        pub fn new(spi: Spi<'d, Blocking>) -> Self {
            Self { spi }
        }

        pub fn build_spi(
            spi2: esp_hal::peripherals::SPI2<'d>,
            sck: impl PeripheralOutput<'d>,
            cs: impl PeripheralOutput<'d>,
            sio0: impl PeripheralOutput<'d>,
            sio1: impl PeripheralOutput<'d>,
            sio2: impl PeripheralOutput<'d>,
            sio3: impl PeripheralOutput<'d>,
        ) -> Result<Self, ConfigError> {
            let spi = Spi::new(
                spi2,
                Config::default()
                    .with_frequency(Rate::from_mhz(board::LCD_SPI_MHZ))
                    .with_mode(Mode::_0),
            )?
            .with_sck(sck)
            .with_cs(cs)
            .with_sio0(sio0)
            .with_sio1(sio1)
            .with_sio2(sio2)
            .with_sio3(sio3);

            Ok(Self { spi })
        }

        #[inline]
        fn qspi_addr(cmd: u8) -> Address {
            // AD[23:0] = 0x00, cmd, 0x00
            Address::_24Bit((cmd as u32) << 8, DataMode::Single)
        }
    }

    impl<'d> St77916Transport for QspiTransport<'d> {
        fn write_param(&mut self, cmd: u8, data: &[u8]) -> Result<(), ()> {
            self.spi
                .half_duplex_write(
                    DataMode::Single,
                    Command::_8Bit(board::LCD_OPCODE_WRITE_CMD as u16, DataMode::Single),
                    Self::qspi_addr(cmd),
                    0,
                    data,
                )
                .map_err(|_| ())
        }

        fn write_color(&mut self, data: &[u8]) -> Result<(), ()> {
            self.spi
                .half_duplex_write(
                    DataMode::Quad,
                    Command::_8Bit(board::LCD_OPCODE_WRITE_COLOR as u16, DataMode::Single),
                    Self::qspi_addr(0x2C),
                    0,
                    data,
                )
                .map_err(|_| ())
        }
    }
}