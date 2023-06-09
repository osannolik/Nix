use crate::bcd::{Bcd, Decimal, Wrapping};
use crate::nixiedigits::NixiePresentation;

use core::borrow::Borrow;

#[derive(Copy, Clone)]
pub struct TimeUnit<const B: u8, Enc>(Enc);

impl<const B: u8, Enc> TimeUnit<B, Enc>
where
    Enc: From<Decimal> + Into<Decimal> + Copy,
{
    pub fn increment(&mut self) {
        let d: Decimal = self.0.into();
        self.0 = Decimal::new((d.value() + 1) % B).into();
    }

    pub fn decrement(&mut self) {
        let d: Decimal = self.0.into();
        self.0 = if d.value() == 0 {
            Decimal::new(B - 1).into()
        } else {
            Decimal::new(d.value() - 1).into()
        };
    }

    pub fn encoding(&self) -> Enc {
        self.0
    }
}

impl<const B: u8, Enc: Wrapping<u8>> Wrapping<u8> for TimeUnit<B, Enc> {
    fn value(&self) -> u8 {
        self.0.value()
    }

    fn set(&mut self, value: u8) {
        self.0.set(value);
    }
}

#[derive(Copy, Clone)]
pub struct Time<Enc> {
    pub seconds: TimeUnit<60, Enc>,
    pub minutes: TimeUnit<60, Enc>,
    pub hours: TimeUnit<24, Enc>,
}

impl<Enc: Wrapping<u8>> Time<Enc> {
    pub fn new(seconds: Enc, minutes: Enc, hours: Enc) -> Self {
        Time {
            seconds: TimeUnit(seconds),
            minutes: TimeUnit(minutes),
            hours: TimeUnit(hours),
        }
    }
}

impl<T: Borrow<Time<Decimal>>> From<T> for Time<Bcd> {
    fn from(value: T) -> Self {
        let v = value.borrow();
        Time::new(v.seconds.0.into(), v.minutes.0.into(), v.hours.0.into())
    }
}

impl<T: Borrow<Time<Bcd>>> From<T> for Time<Decimal> {
    fn from(value: T) -> Self {
        let v = value.borrow();
        Time::new(v.seconds.0.into(), v.minutes.0.into(), v.hours.0.into())
    }
}

impl NixiePresentation<4> for Time<Bcd> {
    fn to_digits(&self) -> [Option<u8>; 4] {
        [
            Some(self.minutes.encoding().ones()),
            Some(self.minutes.encoding().tens()),
            Some(self.hours.encoding().ones()),
            Some(self.hours.encoding().tens()),
        ]
    }
}
