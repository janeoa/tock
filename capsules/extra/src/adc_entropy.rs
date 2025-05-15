use core::cell::Cell;
use kernel::hil::adc::{Adc, Client};
use kernel::hil::entropy::Continue;
use kernel::hil::entropy::{Client32, Entropy32};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;
pub struct AdcEntropy<'a, A: Adc<'a>> {
    adc: &'a A,
    channel: &'a A::Channel, // Store the channel reference
    client: OptionalCell<&'a dyn Client32>,
    entropy_value: Cell<u32>,  // Store the current entropy value
    entropy_ready: Cell<bool>, // Track if entropy is ready
}

impl<'a, A: Adc<'a>> AdcEntropy<'a, A> {
    pub fn new(adc: &'a A, channel: &'a A::Channel) -> Self {
        Self {
            adc,
            channel,
            client: OptionalCell::empty(),
            entropy_value: Cell::new(0),
            entropy_ready: Cell::new(false),
        }
    }
}

impl<'a, A: Adc<'a>> Entropy32<'a> for AdcEntropy<'a, A> {
    fn get(&self) -> Result<(), ErrorCode> {
        self.adc.sample(self.channel)
    }

    fn cancel(&self) -> Result<(), ErrorCode> {
        self.adc.stop_sampling()
    }

    fn set_client(&'a self, client: &'a dyn Client32) {
        self.client.set(client);
    }
}

impl<'a, A: Adc<'a>> Client for AdcEntropy<'a, A> {
    fn sample_ready(&self, sample: u16) {
        let current_entropy = self.entropy_value.get();
        let new_entropy = (current_entropy << 8) | (sample as u32 & 0xFF);
        self.entropy_value.set(new_entropy);

        // If we have collected 4 bytes (32 bits) of entropy, notify the client
        if (new_entropy & 0xFF000000) != 0 {
            self.entropy_ready.set(true);
            self.client.map(|client| {
                let mut iter = AdcEntropyIter::new(new_entropy);
                if let Continue::More = client.entropy_available(&mut iter, Ok(())) {
                    self.adc.sample(self.channel).ok();
                } else {
                    self.entropy_value.set(0);
                    self.entropy_ready.set(false);
                }
                self.client.set(client);
            });
        } else {
            // Continue sampling
            self.adc.sample(self.channel).ok();
        }
    }
}

pub struct AdcEntropyIter {
    entropy_value: u32,
    consumed: bool,
}

impl AdcEntropyIter {
    pub fn new(entropy_value: u32) -> Self {
        Self {
            entropy_value,
            consumed: false,
        }
    }
}

impl Iterator for AdcEntropyIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.consumed {
            self.consumed = true;
            Some(self.entropy_value)
        } else {
            None
        }
    }
}
