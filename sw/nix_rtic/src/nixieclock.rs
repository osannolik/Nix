use crate::buttons::Buttons;
use crate::ds3234::DS3234;
use crate::mode::{DigitPair, Mode, Source};
use crate::nixiedigits::NixieDriver;

use stm32l0xx_hal::exti::{Exti, ExtiLine, GpioLine, TriggerEdge};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{pac, spi};

use crate::bcd::Bcd;
use crate::ext::Buffer;
use crate::time::Time;
use stm32l0xx_hal::gpio::gpioa::{PA0, PA1, PA10, PA2, PA3, PA4, PA5, PA6, PA7, PA9};
use stm32l0xx_hal::gpio::gpiob::PB1;
use stm32l0xx_hal::gpio::gpioc::PC14;
use stm32l0xx_hal::gpio::{Analog, Input, OpenDrain, Output, PullUp, PushPull, Speed};
use stm32l0xx_hal::pac::SPI1;
use stm32l0xx_hal::rcc::Config;
use stm32l0xx_hal::spi::Spi;
use stm32l0xx_hal::syscfg::SYSCFG;

type LedPin = PC14<Output<PushPull>>;
type MosiPin = PA7<Output<OpenDrain>>;
type MisoPin = PA6<Analog>;
type ClkPin = PA5<Output<OpenDrain>>;
type RtcCsPin = PA10<Output<OpenDrain>>;
type HvDriverCsPin = PA9<Output<OpenDrain>>;
type SetPin = PA2<Input<PullUp>>;
type UpPin = PA4<Input<PullUp>>;
type DownPin = PA3<Input<PullUp>>;
type Ext0Pin = PA1<Input<PullUp>>;
type Ext1Pin = PA0<Input<PullUp>>;
type Ext2Pin = PB1<Input<PullUp>>;

type SpiBus = Spi<SPI1, (ClkPin, MisoPin, MosiPin)>;

pub struct ExtPins {
    pub cs: Ext0Pin,
    pub clk: Ext1Pin,
    pub mosi: Ext2Pin,
    pub board_led: LedPin,
}

pub struct NixiePeripherals {
    spi: SpiBus,
    rtc: DS3234<RtcCsPin>,
    driver: NixieDriver<HvDriverCsPin>,
    buttons: Buttons<SetPin, UpPin, DownPin>,
}

pub struct NixieClock {
    peripherals: NixiePeripherals,
    mode: Mode,
    ext_time: Option<Buffer>,
}

pub fn setup_peripherals(dp: pac::Peripherals) -> (NixiePeripherals, ExtPins) {
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    let mut board_led = gpioc.pc14.into_push_pull_output();

    let mut rtc_cs = gpioa.pa10.into_open_drain_output();

    let mut hv_cs = gpioa.pa9.into_open_drain_output();

    let set_pin = gpioa.pa2.into_pull_up_input();
    let dec_pin = gpioa.pa3.into_pull_up_input();
    let inc_pin = gpioa.pa4.into_pull_up_input();

    let sck = gpioa
        .pa5
        .into_open_drain_output()
        .set_speed(Speed::VeryHigh); // See errata sheet ES0332 for STM32L011x4
    let miso = gpioa.pa6; //.into_floating_input();
    let mosi = gpioa.pa7.into_open_drain_output();

    let spi = dp
        .SPI1
        .spi((sck, miso, mosi), spi::MODE_1, 200_000.Hz(), &mut rcc);

    hv_cs.set_low().unwrap();
    rtc_cs.set_high().unwrap();
    board_led.set_high().unwrap();

    let nixie_peripherals = NixiePeripherals {
        spi,
        rtc: DS3234::new(rtc_cs),
        driver: NixieDriver::new(hv_cs),
        buttons: Buttons::new(set_pin, inc_pin, dec_pin),
    };

    let ext_pins = ExtPins {
        cs: gpioa.pa1.into_pull_up_input(),
        clk: gpioa.pa0.into_pull_up_input(),
        mosi: gpiob.pb1.into_pull_up_input(),
        board_led,
    };

    let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);
    let mut exti = Exti::new(dp.EXTI);

    let line = GpioLine::from_raw_line(ext_pins.clk.pin_number()).unwrap();
    exti.listen_gpio(
        &mut syscfg,
        ext_pins.clk.port(),
        line,
        TriggerEdge::Rising,
    );

    let line = GpioLine::from_raw_line(ext_pins.cs.pin_number()).unwrap();
    exti.listen_gpio(
        &mut syscfg,
        ext_pins.cs.port(),
        line,
        TriggerEdge::Both,
    );

    (nixie_peripherals, ext_pins)
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

        //self.peripherals.board_led.set_low().unwrap();
    }
}
