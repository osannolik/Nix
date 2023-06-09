#![no_main]
#![no_std]

mod bcd;
mod buttons;
mod ds3234;
mod mode;
mod nixieclock;
mod nixiedigits;
mod temperature;
mod time;

//use panic_rtt_target as _;
//use rtt_target::{rprintln, rtt_init_print};
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics

use rtic::app;

use stm32l0xx_hal::pac;
use systick_monotonic::{fugit::Duration, Systick};

#[app(
    device = stm32l0xx_hal::pac,
    peripherals = true,
    dispatchers = [SPI1],
)]
mod app {
    use super::*;
    use crate::nixieclock::NixieClock;

    // Setting this monotonic as the default
    #[monotonic(binds = SysTick, default = true)]
    type Tonic = Systick<1000>;
    type TonicTime = Duration<u64, 1, 1000>;

    #[local]
    struct Local {
        nixie: NixieClock,
    }

    #[shared]
    struct Shared {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let peripherals: pac::Peripherals = cx.device;

        //rtt_init_print!();
        //rprintln!("init");

        let nixie = NixieClock::new(peripherals);

        let mono = Systick::new(cx.core.SYST, 16_000_000);

        let _ = main::spawn_after(TonicTime::from_ticks(500));

        (Shared {}, Local { nixie }, init::Monotonics(mono))
    }

    #[task(local = [nixie])]
    fn main(cx: main::Context) {
        cx.local.nixie.update();

        let _ = main::spawn_after(TonicTime::from_ticks(100));
    }
}
