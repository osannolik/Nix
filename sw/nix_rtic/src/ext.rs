use crate::board::{ExtPins, ExtiSource};
use embedded_hal::digital::v2::{InputPin, OutputPin};

const CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT: u8 = 0x02;
const BUFFER_LENGTH: usize = 5;

pub type Buffer = [u8; BUFFER_LENGTH];

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

pub struct ExternalTemperature {
    peripherals: ExtPins,
    parser: ParseSpi,
}

impl ExternalTemperature {
    pub fn new(peripherals: ExtPins) -> Self {
        ExternalTemperature {
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

    pub fn on_interrupt(&mut self) -> Option<Buffer> {
        self.peripherals.board_led.set_high().unwrap();
        let time = self.handle_interrupt();
        self.peripherals.board_led.set_low().unwrap();
        time
    }
}
