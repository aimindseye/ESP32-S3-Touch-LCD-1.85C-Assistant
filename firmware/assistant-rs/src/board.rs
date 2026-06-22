pub const BOARD_NAME: &str = "Waveshare ESP32-S3-Touch-LCD-1.85C / 1.85C-BOX";
pub const BOARD_PROFILE_V1_SCHEMATIC: &str = "WAVESHARE_ESP32_S3_TOUCH_LCD_1_85C_V1";
pub const BATTERY_ADC_SCHEMATIC_LABEL: &str = "BAT_ADC(GPIO8)";
pub const BATTERY_ADC_CONFIRMED_TEXT: &str = "GPIO8 CONFIRMED";
pub const BATTERY_ADC_MISSING_TEXT: &str = "SIGNAL MISSING";
pub const BATTERY_ADC_PATH_TEXT: &str = "BAT ADC GPIO8";
pub const BATTERY_SOURCE_UNAVAILABLE: &str = "UNAVAILABLE";
pub const BATTERY_SOURCE_C_SHIM_VENDOR: &str = "C-SHIM GPIO8 VENDOR";
pub const BATTERY_SOURCE_RUST_DIAG: &str = "RUST GPIO8 DIAG";
pub const BATTERY_VENDOR_ATTEN_TEXT: &str = "ADC_ATTEN_DB_12";

// Vendor demo board map extracted from the uploaded Arduino and ESP-IDF trees.
pub const I2C_SCL_GPIO: u8 = 10;
pub const I2C_SDA_GPIO: u8 = 11;
pub const TOUCH_INT_GPIO: u8 = 4;
pub const BACKLIGHT_GPIO: u8 = 5;
pub const POWER_BUTTON_GPIO: u8 = 6;
pub const BATTERY_ADC_GPIO: u8 = 8;

pub const SD_CLK_GPIO: u8 = 14;
pub const SD_CMD_GPIO: u8 = 17;
pub const SD_D0_GPIO: u8 = 16;
pub const SD_D1_GPIO: i8 = -1;
pub const SD_D2_GPIO: i8 = -1;
pub const SD_D3_GPIO: i8 = -1;
pub const SD_CARD_DEMO_D3_GPIO: u8 = 21; // CONFIG_SD_Card_D3 in vendor SD_MMC.h

pub const QSPI_CS_GPIO: u8 = 21;
pub const QSPI_SCK_GPIO: u8 = 40;
pub const QSPI_D0_GPIO: u8 = 46;
pub const QSPI_D1_GPIO: u8 = 45;
pub const QSPI_D2_GPIO: u8 = 42;
pub const QSPI_D3_GPIO: u8 = 41;
pub const QSPI_TE_GPIO: u8 = 18;

pub const I2S_BCLK_GPIO: u8 = 48;
pub const I2S_LRCLK_GPIO: u8 = 38;
pub const I2S_DOUT_GPIO: u8 = 47;

pub const MIC_BCLK_GPIO: u8 = 15;
pub const MIC_WS_GPIO: u8 = 2;
pub const MIC_DIN_GPIO: u8 = 39;

pub const TCA9554_ADDR: u8 = 0x20;
pub const CST816_ADDR: u8 = 0x15;
pub const PCF85063_ADDR: u8 = 0x51;

// TCA9554 bit positions (0-based in this Rust port).
pub const EXIO_TOUCH_RST: u8 = 0; // Vendor EXIO1
pub const EXIO_LCD_RST: u8 = 1; // Vendor EXIO2
pub const EXIO_SD_CS: u8 = 2; // Vendor EXIO3 if needed for SPI-only fallbacks
pub const EXIO_SAFE_OUTPUT_MASK: u8 = 0x07; // Preserve EXIO0..2 outputs: touch reset, LCD reset, SD CS
pub const EXIO_DISCOVERY_INPUT_MASK: u8 = 0xF8; // Probe EXIO3..7 as inputs only
pub const EXIO_DISCOVERY_CONFIG: u8 = 0xF8; // TCA9554 config bit: 1=input, 0=output

// ST77916 / vendor display profile.
pub const LCD_WIDTH: u16 = 360;
pub const LCD_HEIGHT: u16 = 360;
pub const LCD_COLOR_BITS: u8 = 16;
pub const LCD_SPI_MHZ: u32 = 1;
pub const LCD_SPI_MODE: u8 = 0;
pub const LCD_OPCODE_WRITE_CMD: u8 = 0x02;
pub const LCD_OPCODE_READ_CMD: u8 = 0x0B;
pub const LCD_OPCODE_WRITE_COLOR: u8 = 0x32;

// Backlight profile copied from vendor demo.
pub const BACKLIGHT_MAX: u8 = 100;
pub const BACKLIGHT_DEFAULT: u8 = 70;
pub const BACKLIGHT_PWM_HZ: u32 = 5_000;
pub const BACKLIGHT_PWM_BITS: u8 = 13;
pub const BACKLIGHT_PWM_MAX_DUTY: u16 = (1 << BACKLIGHT_PWM_BITS) - 1;

// Vendor ESP-IDF BAT_Driver.c Measurement_offset=0.994500.
// Battery voltage = calibrated_adc_mv * 3.0 / Measurement_offset.
pub const BATTERY_DIVIDER_SCALE: f32 = 3.0;
pub const BATTERY_MEASUREMENT_OFFSET: f32 = 0.9945;

pub const BOARD_FLASH_MIB: u32 = 16;
pub const BOARD_PSRAM_MIB: u32 = 8;

// The two visible pages in the vendor UI.
pub const UI_TAB_PLACEHOLDER_LEFT: &str = "       ";
pub const UI_TAB_ONBOARD: &str = "Onboard";
pub const UI_TAB_MUSIC: &str = "music";
pub const UI_TAB_PLACEHOLDER_RIGHT: &str = "       ";
