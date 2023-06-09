use core::ops::{Add, Sub};

pub trait Wrapping<T> {
    fn value(&self) -> T;

    fn set(&mut self, value: T);
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Bcd(u8);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Decimal(u8);

impl Wrapping<u8> for Bcd {
    fn value(&self) -> u8 {
        self.0
    }

    fn set(&mut self, value: u8) {
        self.0 = value;
    }
}

impl Wrapping<u8> for Decimal {
    fn value(&self) -> u8 {
        self.0
    }

    fn set(&mut self, value: u8) {
        self.0 = value;
    }
}

impl Add for Decimal {
    type Output = Decimal;

    fn add(self, rhs: Decimal) -> Decimal {
        Decimal(self.0 + rhs.0)
    }
}

impl Add for Bcd {
    type Output = Bcd;

    fn add(self, rhs: Bcd) -> Bcd {
        (Decimal::from(self) + Decimal::from(rhs)).into()
    }
}

impl Sub for Decimal {
    type Output = Decimal;

    fn sub(self, rhs: Decimal) -> Decimal {
        Decimal(self.0 - rhs.0)
    }
}

impl Sub for Bcd {
    type Output = Bcd;

    fn sub(self, rhs: Bcd) -> Bcd {
        (Decimal::from(self) - Decimal::from(rhs)).into()
    }
}

impl Bcd {
    pub fn new(value: u8) -> Self {
        Bcd(value)
    }

    pub fn tens(&self) -> u8 {
        (self.0 & 0xf0) >> 4
    }

    pub fn ones(&self) -> u8 {
        self.0 & 0x0f
    }
}

impl Decimal {
    pub fn new(value: u8) -> Self {
        Decimal(value)
    }
}

impl From<u8> for Decimal {
    fn from(value: u8) -> Self {
        Decimal::new(value)
    }
}

impl From<u8> for Bcd {
    fn from(value: u8) -> Self {
        Bcd::new(value)
    }
}

fn bcd_to_decimal(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0xF)
}

fn decimal_to_bcd(decimal: u8) -> u8 {
    ((decimal / 10) << 4) + (decimal % 10)
}

impl From<Bcd> for Decimal {
    fn from(value: Bcd) -> Self {
        Decimal::new(bcd_to_decimal(value.value()))
    }
}

impl From<Decimal> for Bcd {
    fn from(value: Decimal) -> Self {
        Bcd::new(decimal_to_bcd(value.value()))
    }
}
