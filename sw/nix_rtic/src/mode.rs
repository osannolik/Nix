use crate::buttons::{ButtonStates, PinLevel};

#[derive(Clone, Copy)]
pub enum DigitPair {
    Minutes,
    Hours,
}

#[derive(Clone, Copy)]
pub enum Source {
    Internal,
    External,
}

type Counter = usize;

#[derive(Clone, Copy)]
pub enum Mode {
    DisplayTime,
    DisplayTemp(Source),
    SetTime(DigitPair, Counter, SetTimeMask),
}

#[derive(Clone, Copy)]
pub struct SetTimeMask {
    counter: usize,
    period: usize,
    blank: bool,
}

impl SetTimeMask {
    pub fn mask(&self, digit_pair: &DigitPair) -> [bool; 4] {
        if self.blank {
            match digit_pair {
                DigitPair::Minutes => [false, false, true, true],
                DigitPair::Hours => [true, true, false, false],
            }
        } else {
            [true; 4]
        }
    }

    fn update(&mut self) {
        self.counter += 1;
        if self.counter >= self.period {
            self.counter = 0;
            self.blank = !self.blank;
        }
    }

    fn reset(&mut self) {
        *self = Self::new(self.period);
    }

    fn new(period: usize) -> Self {
        SetTimeMask {
            counter: 0,
            period,
            blank: false,
        }
    }
}

impl Mode {
    pub fn new() -> Mode {
        Mode::DisplayTime
    }

    pub fn next(&mut self, buttons: &ButtonStates) -> Self {
        let set = buttons.set;
        let up_or_down = buttons.down.is_pressed(0) || buttons.up.is_pressed(0);
        match self {
            Mode::DisplayTime => match set.level {
                PinLevel::Falling if set.count < 5 => Mode::DisplayTemp(Source::Internal),
                PinLevel::High if set.count > 10 => {
                    Mode::SetTime(DigitPair::Minutes, 0, SetTimeMask::new(4))
                }
                _ => *self,
            },
            Mode::DisplayTemp(source) => match source {
                Source::Internal if up_or_down => Mode::DisplayTemp(Source::External),
                Source::External if up_or_down => Mode::DisplayTemp(Source::Internal),
                _ => match set.level {
                    PinLevel::Falling if set.count < 5 => Mode::DisplayTime,
                    _ => *self,
                },
            },
            Mode::SetTime(digit_pair, timeout, blanking) => {
                if up_or_down {
                    *timeout = 0;
                    blanking.reset();
                } else {
                    *timeout += 1;
                    blanking.update();
                }

                if *timeout > 50 {
                    Mode::DisplayTime
                } else {
                    match set.level {
                        PinLevel::Falling if set.count < 5 => match digit_pair {
                            DigitPair::Minutes => Mode::SetTime(DigitPair::Hours, 0, *blanking),
                            DigitPair::Hours => Mode::SetTime(DigitPair::Minutes, 0, *blanking),
                        },
                        _ => *self,
                    }
                }
            }
        }
    }
}
