use crate::bcd::Bcd;
use crate::board::NixiePeripherals;
use crate::ext::Buffer;
use crate::mode::{DigitPair, Mode, Source};
use crate::time::Time;

pub struct NixieClock {
    peripherals: NixiePeripherals,
    mode: Mode,
    ext_time: Option<Buffer>,
}

impl NixieClock {
    pub fn new(peripherals: NixiePeripherals) -> NixieClock {
        let mut peripherals = peripherals;

        peripherals.driver.clear(&mut peripherals.spi);

        NixieClock {
            peripherals,
            mode: Mode::new(),
            ext_time: None,
        }
    }

    fn display_current_time(&mut self) {
        let time = self.peripherals.rtc.read_time(&mut self.peripherals.spi);
        self.peripherals
            .driver
            .put(&time, &mut self.peripherals.spi);
    }

    fn display_current_temperature(&mut self) {
        let temperature = self
            .peripherals
            .rtc
            .read_temperature(&mut self.peripherals.spi);
        self.peripherals
            .driver
            .put(&temperature, &mut self.peripherals.spi);
    }

    fn put_ext_time(&mut self, data: &Buffer) {
        let t = Time::new(Bcd::new(0), Bcd::new(data[4]), Bcd::new(data[3]));

        self.peripherals.driver.put(&t, &mut self.peripherals.spi);
    }

    pub fn update(&mut self, ext_data: &Option<Buffer>) {
        use crate::bcd::Wrapping;

        if ext_data.is_some() {
            self.ext_time = *ext_data;
        }

        let buttons = self.peripherals.buttons.poll_state();

        self.mode = self.mode.next(&buttons);

        match self.mode {
            Mode::DisplayTime => {
                self.display_current_time();
            }
            Mode::DisplayTemp(source) => match source {
                Source::Internal => {
                    self.display_current_temperature();
                }
                Source::External => {
                    if let Some(data) = self.ext_time {
                        self.put_ext_time(&data);
                    } else {
                        self.peripherals.driver.clear(&mut self.peripherals.spi);
                    }
                }
            },
            Mode::SetTime(digit_pair, _, blanking) => {
                let mut time = self.peripherals.rtc.read_time(&mut self.peripherals.spi);

                if buttons.up.is_pressed(0) {
                    match digit_pair {
                        DigitPair::Minutes => {
                            time.minutes.increment();
                        }
                        DigitPair::Hours => {
                            time.hours.increment();
                        }
                    }
                    time.seconds.set(0);
                } else if buttons.down.is_pressed(0) {
                    match digit_pair {
                        DigitPair::Minutes => {
                            time.minutes.decrement();
                        }
                        DigitPair::Hours => {
                            time.hours.decrement();
                        }
                    }
                    time.seconds.set(0);
                }

                self.peripherals
                    .rtc
                    .write_time(&time, &mut self.peripherals.spi);

                let mask = blanking.mask(&digit_pair);

                self.peripherals
                    .driver
                    .put_masked(&time, &mask, &mut self.peripherals.spi);
            }
        }
    }
}
