use embedded_hal::i2c::I2c;

#[derive(Debug, Clone, Copy, Default)]
pub struct DateTime {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u8,
}

#[derive(Debug)]
pub enum Error<E> {
    Bus(E),
}

pub struct Pcf85063;

impl Pcf85063 {
    pub const SECONDS: u8 = 0x04;

    pub const fn new() -> Self {
        Self
    }

    pub fn ping<I2C>(&self, i2c: &mut I2C, addr: u8) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        i2c.write(addr, &[]).map_err(Error::Bus)
    }

    pub fn read_datetime<I2C>(
        &self,
        i2c: &mut I2C,
        addr: u8,
    ) -> Result<DateTime, Error<I2C::Error>>
    where
        I2C: I2c,
    {
        let mut buf = [0u8; 7];
        i2c.write_read(addr, &[Self::SECONDS], &mut buf)
            .map_err(Error::Bus)?;

        Ok(DateTime {
            second: bcd_to_dec(buf[0] & 0x7F),
            minute: bcd_to_dec(buf[1] & 0x7F),
            hour: bcd_to_dec(buf[2] & 0x3F),
            day: bcd_to_dec(buf[3] & 0x3F),
            month: bcd_to_dec(buf[5] & 0x1F),
            year: bcd_to_dec(buf[6]),
        })
    }
}

fn bcd_to_dec(v: u8) -> u8 {
    ((v >> 4) * 10) + (v & 0x0F)
}
