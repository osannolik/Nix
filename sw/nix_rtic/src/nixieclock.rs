use crate::board::NixiePeripherals;
use crate::ext::ExternalData;
use crate::mode::{DigitPair, Mode, Source};

pub struct NixieClock {
    peripherals: NixiePeripherals,
    mode: Mode,
}

impl NixieClock {
    pub fn new(peripherals: NixiePeripherals) -> NixieClock {
        let mut peripherals = peripherals;

        peripherals.driver.clear(&mut peripherals.spi);

        NixieClock {
            peripherals,
            mode: Mode::new(),
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

    pub fn update(&mut self, external_data: &Option<ExternalData>) {
        use crate::bcd::Wrapping;

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
                    if let Some(external) = external_data {
                        self.peripherals
                            .driver
                            .put(&external.temperature, &mut self.peripherals.spi);
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
