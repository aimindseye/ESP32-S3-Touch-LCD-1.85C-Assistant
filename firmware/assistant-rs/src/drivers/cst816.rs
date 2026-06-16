use embedded_hal::i2c::I2c;

#[derive(Debug, Clone, Copy, Default)]
pub struct TouchPoint {
    pub gesture: u8,
    pub fingers: u8,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TouchConfig {
    pub version: u8,
    pub chip_id: u8,
    pub project_id: u8,
    pub fw_version: u8,
}

#[derive(Debug)]
pub enum Error<E> {
    Bus(E),
}

pub struct Cst816;

impl Cst816 {
    pub const REG_GESTURE_ID: u8 = 0x01;
    pub const REG_VERSION: u8 = 0x15;
    pub const REG_CHIP_ID: u8 = 0xA7;
    pub const REG_PROJECT_ID: u8 = 0xA8;
    pub const REG_FW_VERSION: u8 = 0xA9;
    pub const REG_DISABLE_AUTO_SLEEP: u8 = 0xFE;

    pub const fn new() -> Self {
        Self
    }

    pub fn ping<I2C>(&self, i2c: &mut I2C, addr: u8) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        i2c.write(addr, &[]).map_err(Error::Bus)
    }

    pub fn read_touch<I2C>(
        &self,
        i2c: &mut I2C,
        addr: u8,
    ) -> Result<TouchPoint, Error<I2C::Error>>
    where
        I2C: I2c,
    {
        let mut buf = [0u8; 6];
        i2c.write_read(addr, &[Self::REG_GESTURE_ID], &mut buf)
            .map_err(Error::Bus)?;

        Ok(TouchPoint {
            gesture: buf[0],
            fingers: buf[1],
            x: (((buf[2] & 0x0F) as u16) << 8) | (buf[3] as u16),
            y: (((buf[4] & 0x0F) as u16) << 8) | (buf[5] as u16),
        })
    }

    pub fn read_config<I2C>(
        &self,
        i2c: &mut I2C,
        addr: u8,
    ) -> Result<TouchConfig, Error<I2C::Error>>
    where
        I2C: I2c,
    {
        let mut version = [0u8; 1];
        let mut chip_id = [0u8; 1];
        let mut project_id = [0u8; 1];
        let mut fw_version = [0u8; 1];

        i2c.write_read(addr, &[Self::REG_VERSION], &mut version)
            .map_err(Error::Bus)?;
        i2c.write_read(addr, &[Self::REG_CHIP_ID], &mut chip_id)
            .map_err(Error::Bus)?;
        i2c.write_read(addr, &[Self::REG_PROJECT_ID], &mut project_id)
            .map_err(Error::Bus)?;
        i2c.write_read(addr, &[Self::REG_FW_VERSION], &mut fw_version)
            .map_err(Error::Bus)?;

        Ok(TouchConfig {
            version: version[0],
            chip_id: chip_id[0],
            project_id: project_id[0],
            fw_version: fw_version[0],
        })
    }

    pub fn disable_auto_sleep<I2C>(
        &self,
        i2c: &mut I2C,
        addr: u8,
    ) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        i2c.write(addr, &[Self::REG_DISABLE_AUTO_SLEEP, 10])
            .map_err(Error::Bus)
    }
}
