#![allow(dead_code)]

#[cfg(feature = "wifi-scan")]
use esp_alloc as _;

#[cfg(feature = "wifi-scan")]
use esp_hal::{peripheral::Peripheral, rng::Rng};

#[cfg(feature = "wifi-scan")]
use esp_wifi::{
    self,
    wifi::{Configuration, ScanConfig, WifiController, WifiDevice, WifiError, WifiMode},
    EspWifiController,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct WifiScanSummary {
    pub scanned_count: u16,
    pub strongest_rssi: Option<i8>,
}

#[cfg(feature = "wifi-scan")]
pub struct WifiScanService<'d> {
    controller: WifiController<'d>,
    _device: WifiDevice<'d>,
    last: WifiScanSummary,
}

#[cfg(feature = "wifi-scan")]
impl<'d> WifiScanService<'d> {
    pub fn new(
        init: &'d EspWifiController<'d>,
        wifi: impl Peripheral<P = esp_hal::peripherals::WIFI> + 'd,
    ) -> Result<Self, WifiError> {
        let (controller, device) = esp_wifi::wifi::new(init, wifi)?;
        Ok(Self {
            controller,
            _device: device,
            last: WifiScanSummary::default(),
        })
    }

    pub fn init_controller(
        timer: impl Peripheral<P = esp_hal::timer::timg::Timer0<esp_hal::timer::timg::TimerGroup<'d, esp_hal::peripherals::TIMG0>>> + 'd,
        rng: impl Peripheral<P = esp_hal::peripherals::RNG> + 'd,
        radio_clk: impl Peripheral<P = esp_hal::peripherals::RADIO_CLK> + 'd,
    ) -> Result<EspWifiController<'d>, esp_wifi::InitializationError> {
        esp_wifi::init(timer, Rng::new(rng), radio_clk)
    }

    pub fn start_station(&mut self) -> Result<(), WifiError> {
        self.controller.set_mode(WifiMode::Sta)?;
        self.controller.set_configuration(&Configuration::Client(Default::default()))?;
        self.controller.start()
    }

    pub fn scan<const N: usize>(&mut self) -> Result<WifiScanSummary, WifiError> {
        let (aps, count) = self.controller.scan_with_config_sync::<N>(ScanConfig::default())?;
        let strongest = aps.iter().map(|ap| ap.signal_strength).max();
        let summary = WifiScanSummary {
            scanned_count: count as u16,
            strongest_rssi: strongest,
        };
        self.last = summary;
        Ok(summary)
    }

    pub fn last_summary(&self) -> WifiScanSummary {
        self.last
    }
}
