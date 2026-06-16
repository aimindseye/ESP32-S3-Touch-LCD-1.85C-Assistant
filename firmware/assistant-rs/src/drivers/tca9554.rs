use embedded_hal::i2c::I2c;

#[derive(Debug, Clone, Copy)]
pub enum Error<E> {
    Bus(E),
    InvalidPin,
}

pub struct Tca9554 {
    output_state: u8,
}

impl Tca9554 {
    pub const OUTPUT_PORT: u8 = 0x01;
    pub const CONFIG: u8 = 0x03;

    pub const fn new() -> Self {
        Self { output_state: 0xFF }
    }

    pub fn ping<I2C>(&self, i2c: &mut I2C, addr: u8) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        i2c.write(addr, &[]).map_err(Error::Bus)
    }

    pub fn set_config<I2C>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        config: u8,
    ) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        i2c.write(addr, &[Self::CONFIG, config]).map_err(Error::Bus)
    }

    pub fn set_output_port<I2C>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        value: u8,
    ) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        self.output_state = value;
        i2c.write(addr, &[Self::OUTPUT_PORT, value])
            .map_err(Error::Bus)
    }

    pub fn write_pin<I2C>(
        &mut self,
        i2c: &mut I2C,
        addr: u8,
        pin: u8,
        high: bool,
    ) -> Result<(), Error<I2C::Error>>
    where
        I2C: I2c,
    {
        if pin > 7 {
            return Err(Error::InvalidPin);
        }

        if high {
            self.output_state |= 1 << pin;
        } else {
            self.output_state &= !(1 << pin);
        }

        self.set_output_port(i2c, addr, self.output_state)
    }
}
