#![allow(dead_code)]

/// Optional esp-radio based Wi-Fi scan path.
///
/// This exists because the user explicitly asked for `esp-radio` / network
/// support parity. The current Rust Wi-Fi ecosystem is in transition, so this
/// file keeps the app-facing contract (`scan count`) separate from the exact
/// radio/runtime bootstrap sequence.
#[cfg(feature = "radio-wifi")]
pub struct RadioWifiScanService;

#[cfg(feature = "radio-wifi")]
impl RadioWifiScanService {
    pub fn start_scheduler() {
        // Expected bootstrap shape from current docs:
        //
        // let timg0 = TimerGroup::new(peripherals.TIMG0);
        // let sw = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
        // esp_rtos::start(timg0.timer0);
        // esp_rtos::start_second_core(sw.software_interrupt0, sw.software_interrupt1, || {});
        //
        // let radio = esp_radio::init().unwrap();
        //
        // From there, bind the radio Wi-Fi controller and expose the same scan
        // summary model used by the vendor demo.
    }
}
