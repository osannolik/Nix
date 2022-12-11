#![no_main]
#![no_std]

//extern crate panic_halt;
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

use cortex_m_rt::entry;
use stm32l0xx_hal::gpio::Speed;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config, spi};

mod buttons;
mod ds3234;
mod nixiedigits;
mod temperature;
mod time;

use crate::buttons::{ButtonState, PinLevel};
use crate::ds3234::DS3234;
use crate::nixiedigits::{NixieDriver, NixiePresentation};
use crate::time::Time;

#[derive(Clone, Copy)]
enum DigitPair {
    Minutes,
    Hours,
}

#[derive(Clone, Copy)]
enum Mode {
    DisplayTime,
    DisplayTemp,
    SetTime(DigitPair),
}

impl Mode {
    fn new() -> Mode {
        Mode::DisplayTime
    }

    fn update(&mut self, set: &ButtonState, set_timeout: &mut Counter) {
        *self = match self {
            Mode::DisplayTime => match set.level {
                PinLevel::Falling if set.count < 5 => Mode::DisplayTemp,
                PinLevel::High if set.count > 10 => Mode::SetTime(DigitPair::Minutes),
                _ => *self,
            },
            Mode::DisplayTemp => match set.level {
                PinLevel::Falling if set.count < 5 => Mode::DisplayTime,
                _ => *self,
            },
            Mode::SetTime(digit_pair) => {
                if set_timeout.finished() {
                    Mode::DisplayTime
                } else {
                    match set.level {
                        PinLevel::Falling if set.count < 5 => {
                            match digit_pair {
                                DigitPair::Minutes => Mode::SetTime(DigitPair::Hours),
                                DigitPair::Hours => Mode::SetTime(DigitPair::Minutes),
                            }
                        },
                        _ => *self,
                    }
                }
            },
        };
    }
}

/*
struct NixieController {
    mode: Mode,
}
impl NixieController {
    pub fn new() -> NixieController {
        NixieController {mode: Mode::new()}
    }

    fn update(&mut self, set: &ButtonState, inc: &ButtonState, dec: &ButtonState) {
        self.mode.update(set);

        match self.mode { }
    }
}
 */

struct Counter {
    pub counter: u32,
    pub interval: u32,
    toggle: bool,
}

impl Counter {
    pub fn new(interval: u32) -> Counter {
        Counter { counter: 0, interval, toggle: false }
    }

    pub fn finished(&mut self) -> bool {
        self.counter += 1;
        if self.counter >= self.interval {
            self.counter = 0;
            return true;
        }
        return false;
    }

    pub fn toggled(&mut self) -> bool {
        if self.finished() {
            self.toggle = !self.toggle;
        }
        self.toggle
    }
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let mut delay = cp.SYST.delay(rcc.clocks);

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    // Configure PA1 as output.
    let mut led = gpioc.pc14.into_push_pull_output();

    let mut hv_cs = gpioa.pa9.into_open_drain_output();
    hv_cs.set_low().unwrap();

    let mut rtc_cs = gpioa.pa10.into_open_drain_output();
    rtc_cs.set_high().unwrap();

    let sck = gpioa
        .pa5
        .into_open_drain_output()
        .set_speed(Speed::VeryHigh); // See errata sheet ES0332 for STM32L011x4
    let miso = gpioa.pa6; //.into_floating_input();
    let mosi = gpioa.pa7.into_open_drain_output();

    let mut spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_1, 200_000.Hz(), &mut rcc);
    //let mut spi = SpiBitBanged::new(mosi, miso, sck);

    let set_pin = gpioa.pa2.into_pull_up_input();
    let dec_pin = gpioa.pa3.into_pull_up_input();
    let inc_pin = gpioa.pa4.into_pull_up_input();

    let mut set_button = ButtonState::new(&set_pin);
    let mut inc_button = ButtonState::new(&inc_pin);
    let mut dec_button = ButtonState::new(&dec_pin);

    //let mut btn = ButtonState2::new();
    let mut mode = Mode::new();

    let mut rtc = DS3234::new(rtc_cs);
    let mut nix = NixieDriver::new(hv_cs);

    let mut blinking = Counter::new(3);
    let mut set_timeout = Counter::new(50);

    let init_time = Time {
        seconds: 0,
        minutes: 37,
        hours: 13,
    };

    nix.put(&init_time, &mut spi);
    rtc.write_time(&init_time, &mut spi);

    delay.delay_ms(1000 as u16);

    loop {
        led.set_high().unwrap();
        delay.delay_ms(20_u16);

        led.set_low().unwrap();
        delay.delay_ms(80_u16);

        //btn.update(&button_set, &button_inc, &button_dec);

        set_button.update(&set_pin);
        inc_button.update(&inc_pin);
        dec_button.update(&dec_pin);

        mode.update(&set_button, &mut set_timeout);

        let mut time = rtc.read_time(&mut spi);

        match mode {
            Mode::SetTime(digit_pair) => {
                match digit_pair {
                    DigitPair::Minutes => {
                        if inc_button.is_pressed(0) {
                            if time.minutes < 59 {
                                time.minutes += 1;
                            } else {
                                time.minutes = 0;
                            }
                            time.seconds = 0;
                            rtc.write_time(&time, &mut spi);
                            set_timeout.counter = 0;
                        } else if dec_button.is_pressed(0) {
                            if time.minutes > 0 {
                                time.minutes -= 1;
                            } else {
                                time.minutes = 59;
                            }
                            time.seconds = 0;
                            rtc.write_time(&time, &mut spi);
                            set_timeout.counter = 0;
                        }
                    }
                    DigitPair::Hours => {
                        if inc_button.is_pressed(0) {
                            if time.hours < 23 {
                                time.hours += 1;
                            } else {
                                time.hours = 0;
                            }
                            time.seconds = 0;
                            rtc.write_time(&time, &mut spi);
                            set_timeout.counter = 0;
                        } else if dec_button.is_pressed(0) {
                            if time.hours > 0 {
                                time.hours -= 1;
                            } else {
                                time.hours = 23;
                            }
                            time.seconds = 0;
                            rtc.write_time(&time, &mut spi);
                            set_timeout.counter = 0;
                        }
                    }
                }
            }
            _ => {}
        }

        match mode {
            Mode::DisplayTime => {
                nix.put(&time, &mut spi);
            }
            Mode::SetTime(digit_pair) => {
                let mut digits = time.to_digits();
                if blinking.toggled() {
                    match digit_pair {
                        DigitPair::Minutes => {
                            digits[0] = None;
                            digits[1] = None;
                        }
                        DigitPair::Hours => {
                            digits[2] = None;
                            digits[3] = None;
                        }
                    }
                }
                nix.put_digits(&digits, &mut spi);
            }
            Mode::DisplayTemp => {
                let temperature = rtc.read_temperature(&mut spi);
                nix.put(&temperature, &mut spi);
            }
        }
    }
}

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
