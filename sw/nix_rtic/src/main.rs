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
    use crate::board::{setup_peripherals, ExtPins};
    use crate::ext::{Buffer, ParseSpi};
    use crate::nixieclock::NixieClock;
    use stm32l0xx_hal::exti::{Exti, ExtiLine, GpioLine};
    use stm32l0xx_hal::prelude::{InputPin, OutputPin};
    use systick_monotonic::fugit::ExtU32;

    // Setting this monotonic as the default
    #[monotonic(binds = SysTick, default = true)]
    type Tonic = Systick<1000>;
    type TonicTime = Duration<u64, 1, 1000>;

    #[local]
    struct Local {
        nixie: NixieClock,
        ext_pins: ExtPins,
        parser: ParseSpi,
    }

    #[shared]
    struct Shared {
        result: Option<Buffer>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        let dp: pac::Peripherals = cx.device;

        //        rtt_init_print!();
        //        rprintln!("hello");

        let (nixie_peripherals, mut ext_pins) = setup_peripherals(dp);

        ext_pins.board_led.set_low().unwrap();

        let nixie = NixieClock::new(nixie_peripherals);

        let parser = ParseSpi::Idle;
        let mono = Systick::new(cx.core.SYST, 16_000_000);

        let _ = main::spawn_after(TonicTime::from_ticks(500));

        (
            Shared { result: None },
            Local {
                nixie,
                ext_pins,
                parser,
            },
            init::Monotonics(mono),
        )
    }

    #[task(priority = 1, local = [nixie], shared = [result])]
    fn main(mut ctx: main::Context) {
        let next_time = monotonics::now() + 100.millis();

        let mut ext_data: Option<Buffer> = None;
        ctx.shared.result.lock(|r| {
            ext_data = *r;
            *r = None;
        });

        ctx.local.nixie.update(&ext_data);

        let _ = main::spawn_at(next_time);
    }

    #[task(priority = 2, binds = EXTI0_1, local = [ext_pins, parser], shared = [result])]
    fn exti_interrupt(mut ctx: exti_interrupt::Context) {
        let exti_interrupt::LocalResources { ext_pins, parser } = ctx.local;
        ext_pins.board_led.set_high().unwrap();

        let data_is_high = ext_pins.mosi.is_high().unwrap();

        let clk_line = GpioLine::from_raw_line(ext_pins.clk.pin_number()).unwrap();
        if Exti::is_pending(clk_line) {
            parser.on_clk_rising_edge(data_is_high);

            Exti::unpend(clk_line);
        }

        let cs_line = GpioLine::from_raw_line(ext_pins.cs.pin_number()).unwrap();
        if Exti::is_pending(cs_line) {
            let is_high = ext_pins.cs.is_high().unwrap();

            if let Some(x) = parser.on_cs_edges(is_high) {
                ctx.shared.result.lock(|r| {
                    *r = Some(x);
                });
            }

            Exti::unpend(cs_line);
        }

        ext_pins.board_led.set_low().unwrap();
    }
}
