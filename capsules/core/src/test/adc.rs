use core::cell::Cell;
use kernel::debug;
use kernel::hil::adc::{Adc, Client};
pub struct TestAdc<'a, A: Adc<'a>> {
    adc: &'a A,
}

impl<'a, A: Adc<'a>> TestAdc<'a, A> {
    pub fn new(adc: &'a A) -> TestAdc<'a, A> {
        TestAdc { adc }
    }

    pub fn run(&self) {
        debug!("Starting adc!");
    }
}

impl<'a, A: Adc<'a>> Client for TestAdc<'a, A> {
    fn sample_ready(&self, sample: u16) {}
}
