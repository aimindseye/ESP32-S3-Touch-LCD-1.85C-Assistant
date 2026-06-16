#![allow(dead_code)]

use crate::board;
use embedded_hal::pwm::SetDutyCycle;
use esp_hal::{
    gpio::{interconnect::PeripheralOutput, DriveMode},
    ledc::{
        channel::{self, Channel, ChannelIFace},
        timer::{self, Timer, TimerIFace},
        LSGlobalClkSource, Ledc, LowSpeed,
    },
    time::Rate,
};

#[derive(Debug)]
pub enum BacklightError {
    Timer(timer::Error),
    Channel(channel::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct BacklightState {
    pub percent: u8,
    pub raw_duty: u16,
}

pub struct Backlight<'a> {
    channel: Channel<'a, LowSpeed>,
    state: BacklightState,
}

impl<'a> Backlight<'a> {
    pub fn configure_ledc(ledc: &mut Ledc<'a>) {
        ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    }

    pub fn configure_timer(ledc: &Ledc<'a>) -> Result<Timer<'a, LowSpeed>, timer::Error> {
        let mut timer0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
        timer0.configure(timer::config::Config {
            duty: timer::config::Duty::Duty13Bit,
            clock_source: timer::LSClockSource::APBClk,
            frequency: Rate::from_hz(board::BACKLIGHT_PWM_HZ),
        })?;
        Ok(timer0)
    }

    pub fn new(
        ledc: &Ledc<'a>,
        pin: impl PeripheralOutput<'a>,
        timer: &'a Timer<'a, LowSpeed>,
    ) -> Result<Self, BacklightError> {
        let mut channel0 = ledc.channel(channel::Number::Channel0, pin);
        channel0
            .configure(channel::config::Config {
                timer,
                duty_pct: 0,
                drive_mode: DriveMode::PushPull,
            })
            .map_err(BacklightError::Channel)?;

        let mut this = Self {
            channel: channel0,
            state: BacklightState {
                percent: 0,
                raw_duty: 0,
            },
        };

        this.set_percent(board::BACKLIGHT_DEFAULT)
            .map_err(BacklightError::Channel)?;

        Ok(this)
    }

    pub fn set_percent(&mut self, light: u8) -> Result<(), channel::Error> {
        let clamped = if light > board::BACKLIGHT_MAX {
            board::BACKLIGHT_MAX
        } else {
            light
        };
        let raw = vendor_backlight_raw_duty(clamped);

        self.state = BacklightState {
            percent: clamped,
            raw_duty: raw,
        };

        self.channel.set_duty_cycle(raw)
    }

    pub fn percent(&self) -> u8 {
        self.state.percent
    }

    pub fn state(&self) -> BacklightState {
        self.state
    }
}

pub const fn vendor_backlight_raw_duty(light: u8) -> u16 {
    if light == 0 {
        0
    } else {
        let capped = if light > board::BACKLIGHT_MAX {
            board::BACKLIGHT_MAX
        } else {
            light
        };
        board::BACKLIGHT_PWM_MAX_DUTY - (81 * (board::BACKLIGHT_MAX - capped) as u16)
    }
}