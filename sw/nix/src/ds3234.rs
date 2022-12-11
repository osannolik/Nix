use core::fmt::Debug;
use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::digital::v2::OutputPin;

use crate::temperature::Temperature;
use crate::time::Time;

pub struct DS3234<PinCS> {
    cs: PinCS,
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Registers {
    Seconds = 0x00,
    Minutes = 0x01,
    Hours = 0x02,
    Day = 0x03,
    Date = 0x04,
    Month = 0x05,
    Year = 0x06,
    Alarm1Seconds = 0x07,
    Alarm1Minutes = 0x08,
    Alarm1Hours = 0x09,
    Alarm1DayDate = 0x0a,
    Alarm2Minutes = 0xb,
    Alarm2Hours = 0x0c,
    Alarm2DayDate = 0x0d,
    Control = 0x0e,
    Status = 0x0f,
    AgingOffset = 0x10,
    TemperatureMsb = 0x11,
    TemperatureLsb = 0x12,
    TemperatureConv = 0x13,
}

impl Registers {
    const SPI_WRITE_BIT: u8 = 0x80;

    pub fn read(&self) -> u8 {
        *self as u8
    }
    pub fn write(&self) -> u8 {
        Self::SPI_WRITE_BIT | self.read()
    }
}

struct RegisterBits;

#[allow(dead_code)]
impl RegisterBits {
    const MASK_HOURS: u8 = 0b0011_1111;
    const H24_H12: u8 = 0b0100_0000;
    const AM_PM: u8 = 0b0010_0000;
    const CENTURY: u8 = 0b1000_0000;
    const EOSC: u8 = 0b1000_0000;
    const BBSQW: u8 = 0b0100_0000;
    const TEMP_CONV: u8 = 0b0010_0000;
    const RS2: u8 = 0b0001_0000;
    const RS1: u8 = 0b0000_1000;
    const INTCN: u8 = 0b0000_0100;
    const ALARM2_INT_EN: u8 = 0b0000_0010;
    const ALARM1_INT_EN: u8 = 0b0000_0001;
    const OSC_STOP: u8 = 0b1000_0000;
    const BB32KHZ: u8 = 0b0100_0000;
    const CRATE1: u8 = 0b0010_0000;
    const CRATE0: u8 = 0b0001_0000;
    const EN32KHZ: u8 = 0b0000_1000;
    const BUSY: u8 = 0b0000_0100;
    const ALARM2F: u8 = 0b0000_0010;
    const ALARM1F: u8 = 0b0000_0001;
    const TEMP_CONV_BAT: u8 = 0b0000_0001;
    const ALARM_MATCH: u8 = 0b1000_0000;
    const WEEKDAY: u8 = 0b0100_0000;
}

fn bcd_to_decimal(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0xF)
}

pub fn decimal_to_bcd(decimal: u8) -> u8 {
    ((decimal / 10) << 4) + (decimal % 10)
}

impl<PinCS, PinE> DS3234<PinCS>
where
    PinE: Debug,
    PinCS: OutputPin<Error = PinE>,
{
    pub fn new(cs: PinCS) -> Self {
        let mut rtc = DS3234 { cs };
        rtc.cs.set_high().unwrap();
        rtc
    }

    fn read_register<Spi, SpiE>(&mut self, spi: &mut Spi, reg: Registers) -> u8
    where
        SpiE: Debug,
        Spi: Transfer<u8, Error = SpiE>,
    {
        let mut tmp: [u8; 2] = [reg.read(), 0];
        self.cs.set_low().unwrap();
        spi.transfer(&mut tmp).unwrap();
        self.cs.set_high().unwrap();
        tmp[1]
    }

    pub fn write_register<Spi, SpiE>(&mut self, spi: &mut Spi, reg: Registers, bits: u8)
    where
        SpiE: Debug,
        Spi: Write<u8, Error = SpiE>,
    {
        self.cs.set_low().unwrap();
        spi.write(&[reg.write(), bits]).unwrap();
        self.cs.set_high().unwrap();
    }

    fn write_data<Spi, SpiE>(&mut self, spi: &mut Spi, reg: Registers, data: &mut [u8])
    where
        SpiE: Debug,
        Spi: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    {
        self.cs.set_low().unwrap();
        spi.write(&[reg.write()]).unwrap();
        spi.transfer(data).unwrap();
        self.cs.set_high().unwrap();
    }

    fn read_data<Spi, SpiE>(&mut self, spi: &mut Spi, reg: Registers, data: &mut [u8])
    where
        SpiE: Debug,
        Spi: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    {
        self.cs.set_low().unwrap();
        spi.write(&[reg.read()]).unwrap();
        spi.transfer(data).unwrap();
        self.cs.set_high().unwrap();
    }

    pub fn read_time<Spi, SpiE>(&mut self, spi: &mut Spi) -> Time
    where
        SpiE: Debug,
        Spi: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    {
        let mut time_data = [0x00; 3];
        self.read_data(spi, Registers::Seconds, &mut time_data);

        return Time {
            seconds: bcd_to_decimal(time_data[0]),
            minutes: bcd_to_decimal(time_data[1]),
            hours: bcd_to_decimal(RegisterBits::MASK_HOURS & time_data[2]),
        };
    }

    pub fn write_time<Spi, SpiE>(&mut self, time: &Time, spi: &mut Spi)
    where
        SpiE: Debug,
        Spi: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    {
        let mut time_data = [
            decimal_to_bcd(time.seconds),
            decimal_to_bcd(time.minutes),
            RegisterBits::MASK_HOURS & decimal_to_bcd(time.hours),
        ];
        self.cs.set_low().unwrap();
        self.write_data(spi, Registers::Seconds, &mut time_data);
        self.cs.set_high().unwrap();
    }

    pub fn read_temperature<Spi, SpiE>(&mut self, spi: &mut Spi) -> Temperature
    where
        SpiE: Debug,
        Spi: Transfer<u8, Error = SpiE> + Write<u8, Error = SpiE>,
    {
        let mut data = [0x00; 2];
        self.read_data(spi, Registers::TemperatureMsb, &mut data);
        let is_negative = (data[0] & 0b1000_0000) != 0;
        let temp = (u16::from(data[0]) << 2) | u16::from(data[1] >> 6);
        if is_negative {
            let temp_sign_extended = temp | 0b1111_1100_0000_0000;
            Temperature(f32::from(temp_sign_extended as i16) * 0.25)
        } else {
            Temperature(f32::from(temp) * 0.25)
        }
    }
}
