use crate::nixiedigits::NixiePresentation;

use num_traits::float::FloatCore;

pub struct Temperature(pub f32);

impl NixiePresentation<4> for Temperature {
    fn to_digits(&self) -> [Option<u8>; 4] {
        if self.0 > 0.0 {
            let int_part = self.0.trunc() as u8;
            let decimal_part = (self.0.fract() * 100.0) as u8;
            let last_digit = decimal_part % 10;
            [
                if last_digit > 0 {
                    Some(last_digit)
                } else {
                    None
                },
                Some(decimal_part / 10),
                Some(int_part % 10),
                Some(int_part / 10),
            ]
        } else {
            [None, None, None, None]
        }
    }
}
