use crate::buttons::Buttons;
use crate::ds3234::DS3234;
use crate::nixiedigits::NixieDriver;

use stm32l0xx_hal::exti::{Exti, ExtiLine, GpioLine, TriggerEdge};
use stm32l0xx_hal::prelude::*;
use stm32l0xx_hal::{pac, spi};

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

#[derive(Copy, Clone)]
pub enum ExtiSource {
    Clock(GpioLine),
    Cs(GpioLine),
}

impl ExtiSource {
    pub fn clear(self) {
        match self {
            ExtiSource::Clock(line) | ExtiSource::Cs(line) => Exti::unpend(line),
        }
    }
}

impl ExtPins {
    fn setup_interrupts(&mut self, syscfg: &mut SYSCFG, exti: &mut Exti) {
        let line = GpioLine::from_raw_line(self.clk.pin_number()).unwrap();
        exti.listen_gpio(syscfg, self.clk.port(), line, TriggerEdge::Rising);

        let line = GpioLine::from_raw_line(self.cs.pin_number()).unwrap();
        exti.listen_gpio(syscfg, self.cs.port(), line, TriggerEdge::Both);
    }

    pub fn interrupt_pending(&self) -> Option<ExtiSource> {
        let line = GpioLine::from_raw_line(self.clk.pin_number()).unwrap();
        if Exti::is_pending(line) {
            return Some(ExtiSource::Clock(line));
        }
        let line = GpioLine::from_raw_line(self.cs.pin_number()).unwrap();
        if Exti::is_pending(line) {
            return Some(ExtiSource::Cs(line));
        }
        None
    }
}

pub struct NixiePeripherals {
    pub spi: SpiBus,
    pub rtc: DS3234<RtcCsPin>,
    pub driver: NixieDriver<HvDriverCsPin>,
    pub buttons: Buttons<SetPin, UpPin, DownPin>,
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

    let mut ext_pins = ExtPins {
        cs: gpioa.pa1.into_pull_up_input(),
        clk: gpioa.pa0.into_pull_up_input(),
        mosi: gpiob.pb1.into_pull_up_input(),
        board_led,
    };

    let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);
    let mut exti = Exti::new(dp.EXTI);

    ext_pins.setup_interrupts(&mut syscfg, &mut exti);

    (nixie_peripherals, ext_pins)
}
