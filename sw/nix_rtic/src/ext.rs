use crate::bcd::{Bcd, BcdDigits};
use crate::board::{ExtPins, ExtiSource};
use crate::nixiedigits::NixiePresentation;
use embedded_hal::digital::v2::{InputPin, OutputPin};

const CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT: u8 = 0x02;
const BUFFER_LENGTH: usize = 5;

type Buffer = [u8; BUFFER_LENGTH];

#[derive(Copy, Clone)]
pub struct ExternalDigits(BcdDigits<2>);

impl NixiePresentation<4> for ExternalDigits {
    fn to_digits(&self) -> [Option<u8>; 4] {
        let pair = self.0;
        [
            pair[0].ones(),
            pair[0].tens(),
            pair[1].ones(),
            pair[1].tens(),
        ]
    }
}

#[derive(Copy, Clone)]
pub struct ExternalData {
    pub temperature: ExternalDigits,
    pub voltage: ExternalDigits,
}

impl From<Buffer> for ExternalData {
    fn from(value: Buffer) -> Self {
        ExternalData {
            temperature: ExternalDigits([Bcd::new(value[4]), Bcd::new(value[3])]),
            voltage: ExternalDigits([Bcd::new(value[2]), Bcd::new(value[1])]),
        }
    }
}

#[derive(Copy, Clone)]
pub enum ParseSpi {
    Idle,
    Collecting(usize, Buffer),
}

impl ParseSpi {
    pub fn on_cs_edges(&mut self, is_high: bool) -> Option<Buffer> {
        let (next, result) = match self {
            ParseSpi::Idle if !is_high => (ParseSpi::Collecting(0, [0; BUFFER_LENGTH]), None),
            ParseSpi::Collecting(_, buffer) if is_high => (ParseSpi::Idle, Some(*buffer)),
            _ => (*self, None),
        };
        *self = next;
        result
    }

    pub fn on_clk_rising_edge(&mut self, data_is_high: bool) {
        match self {
            ParseSpi::Idle => {}
            ParseSpi::Collecting(index, buffer) => {
                let byte_nr: usize = *index / 8;
                buffer[byte_nr] <<= 1;
                buffer[byte_nr] |= data_is_high as u8;
                *index += 1;

                /* Sanity check */
                if (*index == 8) && (buffer[byte_nr] != CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT) {
                    *self = ParseSpi::Idle;
                }
            }
        }
    }
}

pub struct External {
    peripherals: ExtPins,
    parser: ParseSpi,
}

impl External {
    pub fn new(peripherals: ExtPins) -> Self {
        External {
            peripherals,
            parser: ParseSpi::Idle,
        }
    }

    fn handle_interrupt(&mut self) -> Option<Buffer> {
        if let Some(irq) = self.peripherals.interrupt_pending() {
            let time = match irq {
                ExtiSource::Clock(_) => {
                    self.parser
                        .on_clk_rising_edge(self.peripherals.mosi.is_high().unwrap());
                    None
                }
                ExtiSource::Cs(_) => self
                    .parser
                    .on_cs_edges(self.peripherals.cs.is_high().unwrap()),
            };
            irq.clear();
            time
        } else {
            None
        }
    }

    pub fn on_interrupt(&mut self) -> Option<ExternalData> {
        self.peripherals.board_led.set_high().unwrap();
        let time = self.handle_interrupt();
        self.peripherals.board_led.set_low().unwrap();
        time.map(|buffer| buffer.into())
    }
}
