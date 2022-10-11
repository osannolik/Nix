#![no_main]
#![no_std]

//extern crate panic_halt;
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

use cortex_m_rt::entry;
use stm32l0xx_hal::gpio::Speed;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config, spi};

mod ds3234;
mod nixiedigits;
mod temperature;
mod time;

use crate::ds3234::DS3234;
use crate::nixiedigits::NixieDriver;
use crate::time::Time;

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

    let mut rtc = DS3234::new(rtc_cs);
    let mut nix = NixieDriver::new(hv_cs);

    nix.put(
        &Time {
            seconds: 0,
            minutes: 0,
            hours: 0,
        },
        &mut spi,
    );

    delay.delay_ms(1000_u16);

    loop {
        led.set_high().unwrap();
        delay.delay_ms(20_u16);

        led.set_low().unwrap();
        delay.delay_ms(100_u16);

        let time = rtc.read_time(&mut spi);
        let temperature = rtc.read_temperature(&mut spi);

        if time.seconds < 30 {
            nix.put(&time, &mut spi);
        } else {
            nix.put(&temperature, &mut spi);
        }
    }
}

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger
