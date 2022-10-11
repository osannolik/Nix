use crate::nixiedigits::NixiePresentation;

pub struct Time {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
}

impl NixiePresentation<4> for Time {
    fn to_digits(&self) -> [Option<u8>; 4] {
        [
            Some(self.seconds % 10),
            Some(self.seconds / 10),
            Some(self.minutes % 10),
            Some(self.minutes / 10),
        ]
    }
}
