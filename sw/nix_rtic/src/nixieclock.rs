use crate::buttons::Buttons;
use crate::ds3234::DS3234;
use crate::mode::{DigitPair, Mode};
use crate::nixiedigits::NixieDriver;

use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{pac, spi};

use stm32l0xx_hal::gpio::gpioa::{PA10, PA2, PA3, PA4, PA5, PA6, PA7, PA9};
use stm32l0xx_hal::gpio::gpioc::PC14;
use stm32l0xx_hal::gpio::{Analog, Input, OpenDrain, Output, PullUp, PushPull, Speed};
use stm32l0xx_hal::pac::SPI1;
use stm32l0xx_hal::rcc::Config;
use stm32l0xx_hal::spi::Spi;

type LedPin = PC14<Output<PushPull>>;
type MosiPin = PA7<Output<OpenDrain>>;
type MisoPin = PA6<Analog>;
type ClkPin = PA5<Output<OpenDrain>>;
type RtcCsPin = PA10<Output<OpenDrain>>;
type HvDriverCsPin = PA9<Output<OpenDrain>>;
type SetPin = PA2<Input<PullUp>>;
type UpPin = PA4<Input<PullUp>>;
type DownPin = PA3<Input<PullUp>>;

type SpiBus = Spi<SPI1, (ClkPin, MisoPin, MosiPin)>;

pub struct NixieClock {
    spi: SpiBus,
    rtc: DS3234<RtcCsPin>,
    driver: NixieDriver<HvDriverCsPin>,
    buttons: Buttons<SetPin, UpPin, DownPin>,
    mode: Mode,
    board_led: LedPin,
}

impl NixieClock {
    pub fn new(dp: pac::Peripherals) -> NixieClock {
        let mut rcc = dp.RCC.freeze(Config::hsi16());

        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);

        let set_pin = gpioa.pa2.into_pull_up_input();
        let dec_pin = gpioa.pa3.into_pull_up_input();
        let inc_pin = gpioa.pa4.into_pull_up_input();

        let mut board_led = gpioc.pc14.into_push_pull_output();
        board_led.set_high().unwrap();

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

        let mut driver = NixieDriver::new(hv_cs);

        driver.clear(&mut spi);

        NixieClock {
            spi,
            rtc: DS3234::new(rtc_cs),
            driver,
            buttons: Buttons::new(set_pin, inc_pin, dec_pin),
            mode: Mode::new(),
            board_led,
        }
    }

    fn display_current_time(&mut self) {
        let time = self.rtc.read_time(&mut self.spi);
        self.driver.put(&time, &mut self.spi);
    }

    fn display_current_temperature(&mut self) {
        let temperature = self.rtc.read_temperature(&mut self.spi);
        self.driver.put(&temperature, &mut self.spi);
    }

    pub fn update(&mut self) {
        use crate::bcd::Wrapping;

        self.board_led.set_high().unwrap();

        let buttons = self.buttons.poll_state();

        self.mode = self.mode.next(&buttons);

        match self.mode {
            Mode::DisplayTime => {
                self.display_current_time();
            }
            Mode::DisplayTemp => {
                self.display_current_temperature();
            }
            Mode::SetTime(digit_pair, _, blanking) => {
                let mut time = self.rtc.read_time(&mut self.spi);

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

                self.rtc.write_time(&time, &mut self.spi);

                let mask = blanking.mask(&digit_pair);

                self.driver.put_masked(&time, &mask, &mut self.spi);
            }
        }

        self.board_led.set_low().unwrap();
    }
}
