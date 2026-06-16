#![allow(dead_code)]

use crate::board;

#[derive(Debug, Clone, Copy)]
pub struct CardInfo {
    pub capacity_mib: u32,
    pub sector_size: u32,
}

#[derive(Debug)]
pub enum Error {
    MountFailed,
    Transport,
}

pub trait SdHost {
    fn mount(&mut self) -> Result<CardInfo, Error>;
    fn card_info(&self) -> Option<CardInfo>;
}

#[cfg(feature = "sdmmc-host")]
pub mod spi_host {
    use super::{CardInfo, Error, SdHost};
    use embedded_hal::delay::DelayNs;
    use embedded_sdmmc::{SdCard, TimeSource, Timestamp};

    #[derive(Clone, Copy, Default)]
    pub struct DummyTimeSource;

    impl TimeSource for DummyTimeSource {
        fn get_timestamp(&self) -> Timestamp {
            Timestamp {
                year_since_1970: 56,
                zero_indexed_month: 0,
                zero_indexed_day: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            }
        }
    }

    pub struct SpiSdHost<SPI, DELAY> {
        card: SdCard<SPI, DELAY>,
        info: Option<CardInfo>,
    }

    impl<SPI, DELAY> SpiSdHost<SPI, DELAY>
    where
        SPI: embedded_hal::spi::SpiDevice,
        DELAY: DelayNs,
    {
        pub fn new(spi: SPI, delay: DELAY) -> Self {
            Self {
                card: SdCard::new(spi, delay),
                info: None,
            }
        }
    }

    impl<SPI, DELAY> SdHost for SpiSdHost<SPI, DELAY>
    where
        SPI: embedded_hal::spi::SpiDevice,
        DELAY: DelayNs,
    {
        fn mount(&mut self) -> Result<CardInfo, Error> {
            let bytes = self.card.num_bytes().map_err(|_| Error::MountFailed)?;
            let info = CardInfo {
                capacity_mib: (bytes / (1024 * 1024)) as u32,
                sector_size: 512,
            };
            self.info = Some(info);
            Ok(info)
        }

        fn card_info(&self) -> Option<CardInfo> {
            self.info
        }
    }
}

#[cfg(not(feature = "sdmmc-host"))]
pub struct PlaceholderSdHost;

#[cfg(not(feature = "sdmmc-host"))]
impl PlaceholderSdHost {
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "sdmmc-host"))]
impl SdHost for PlaceholderSdHost {
    fn mount(&mut self) -> Result<CardInfo, Error> {
        Err(Error::Transport)
    }

    fn card_info(&self) -> Option<CardInfo> {
        None
    }
}

pub const VENDOR_SD_NOTE: &str = "Vendor firmware uses the native SD/MMC host; this Rust port ships a real SPI-backed SD host as the portable esp-hal implementation path.";
pub const VENDOR_SD_PINS: (u8, u8, u8, u8) = (
    board::SD_CLK_GPIO,
    board::SD_CMD_GPIO,
    board::SD_D0_GPIO,
    board::SD_CARD_DEMO_D3_GPIO,
);
