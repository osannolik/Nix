use core::fmt::Debug;
use embedded_hal::blocking::spi::Write;
use embedded_hal::digital::v2::OutputPin;

type Digits<const N: usize> = [Option<u8>; N];

pub trait NixiePresentation<const N: usize> {
    fn to_digits(&self) -> Digits<N>;
}

pub struct NixieDriver<PinCS> {
    cs: PinCS,
}

mod nixie_io {
    use bitvec::prelude::*;

    const N_DIGIT_PAIRS: usize = 2;
    pub const N_DIGITS: usize = 2 * N_DIGIT_PAIRS;
    const N_VALUES: usize = 10;
    const N_BYTES: usize = N_DIGITS * N_VALUES / u8::BITS as usize;

    const INDEX_OFFSET: [[usize; N_VALUES]; N_DIGITS] = [
        // Offset counted from HVout_20 of last HV5812 in data-chain
        [17, 0, 1, 2, 3, 4, 15, 16, 19, 18],      // TBE1 (A)
        [9, 12, 11, 10, 13, 14, 5, 6, 7, 8],      // TBE2 (B)
        [37, 20, 21, 22, 23, 24, 35, 36, 39, 38], // TBE3 (C)
        [29, 32, 31, 30, 33, 34, 25, 26, 27, 28], // TBE4 (D)
    ];

    pub fn hvdata(digits: &[Option<u8>; N_DIGITS]) -> [u8; N_BYTES] {
        let mut data: [u8; N_BYTES] = [0xff; N_BYTES];

        for (tbe, &value) in digits.iter().enumerate() {
            if let Some(d) = value {
                let index = INDEX_OFFSET[tbe][d as usize];
                data.view_bits_mut::<Msb0>().set(index, false);
            }
        }

        data
    }
}

impl<PinCS, PinE> NixieDriver<PinCS>
where
    PinE: Debug,
    PinCS: OutputPin<Error = PinE>,
{
    pub fn new(cs: PinCS) -> Self {
        let mut driver = NixieDriver { cs };
        driver.cs.set_low().unwrap();
        driver
    }

    pub fn put_digits<Spi, SpiE>(&mut self, digits: &Digits<{ nixie_io::N_DIGITS }>, spi: &mut Spi)
    where
        SpiE: Debug,
        Spi: Write<u8, Error = SpiE>,
    {
        let hvdata = nixie_io::hvdata(&digits);
        spi.write(&hvdata).unwrap();
        self.cs.set_high().unwrap();
        self.cs.set_low().unwrap();
    }

    pub fn clear<Spi, SpiE>(&mut self, spi: &mut Spi)
    where
        SpiE: Debug,
        Spi: Write<u8, Error = SpiE>,
    {
        self.put_digits(&[None; nixie_io::N_DIGITS], spi);
    }

    pub fn put<Info, Spi, SpiE>(&mut self, info: &Info, spi: &mut Spi)
    where
        Info: NixiePresentation<{ nixie_io::N_DIGITS }>,
        SpiE: Debug,
        Spi: Write<u8, Error = SpiE>,
    {
        self.put_digits(&info.to_digits(), spi);
    }
}
