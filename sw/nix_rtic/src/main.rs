#![no_main]
#![no_std]

mod bcd;
mod board;
mod buttons;
mod ds3234;
mod ext;
mod mode;
mod nixieclock;
mod nixiedigits;
mod temperature;
mod time;

//use panic_rtt_target as _;
//use rtt_target::{rprintln, rtt_init_print};
//use cortex_m_semihosting::hprintln;
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
    use crate::board::setup_peripherals;
    use crate::ext::{External, ExternalData};
    use crate::nixieclock::NixieClock;
    use systick_monotonic::fugit::ExtU32;

    // Setting this monotonic as the default
    #[monotonic(binds = SysTick, default = true)]
    type Tonic = Systick<1000>;
    type TonicTime = Duration<u64, 1, 1000>;

    #[local]
    struct Local {
        nixie: NixieClock,
        external: External,
    }

    #[shared]
    struct Shared {
        external_data: Option<ExternalData>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let dp: pac::Peripherals = cx.device;

        //        rtt_init_print!();
        //        rprintln!("hello");

        let (nixie_peripherals, ext_pins) = setup_peripherals(dp);

        let nixie = NixieClock::new(nixie_peripherals);
        let external = External::new(ext_pins);

        let mono = Systick::new(cx.core.SYST, 16_000_000);

        let _ = main::spawn_after(TonicTime::from_ticks(500));

        (
            Shared {
                external_data: None,
            },
            Local { nixie, external },
            init::Monotonics(mono),
        )
    }

    #[task(priority = 1, local = [nixie], shared = [external_data])]
    fn main(mut ctx: main::Context) {
        let next_time = monotonics::now() + 100.millis();

        let mut external_data: Option<ExternalData> = None;
        ctx.shared.external_data.lock(|r| {
            external_data = *r;
        });

        ctx.local.nixie.update(&external_data);

        let _ = main::spawn_at(next_time);
    }

    #[task(priority = 2, binds = EXTI0_1, local = [external], shared = [external_data])]
    fn exti_interrupt(mut ctx: exti_interrupt::Context) {
        //let exti_interrupt::LocalResources { external_temperature } = ctx.local;

        if let Some(data) = ctx.local.external.on_interrupt() {
            ctx.shared.external_data.lock(|r| {
                *r = Some(data);
            });
        }
    }
}
