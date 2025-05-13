use core::cell::Cell;
use kernel::hil::entropy::{Client32, Entropy32};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;

const MOCK_RN: u32 = 0xFF;

pub struct MockEntropy32<'a> {
    client: OptionalCell<&'a dyn Client32>,
    mock_value: Cell<u32>,
}

impl<'a> MockEntropy32<'a> {
    pub fn new() -> Self {
        Self {
            client: OptionalCell::empty(),
            mock_value: Cell::<u32>::new(MOCK_RN),
        }
    }
}

impl<'a> Entropy32<'a> for MockEntropy32<'a> {
    fn get(&self) -> Result<(), ErrorCode> {
        if let Some(client) = self.client.take() {
            client.entropy_available(&mut MockEntropyIter(self), Ok(())); // Provide the next value
        }
        Ok(())
    }

    fn cancel(&self) -> Result<(), ErrorCode> {
        Ok(())
    }

    fn set_client(&'a self, client: &'a dyn Client32) {
        self.client.set(client);
    }
}

pub struct MockEntropyIter<'a, 'b: 'a>(&'a MockEntropy32<'b>);

impl<'a> Iterator for MockEntropyIter<'_, '_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let rn = self.0.mock_value.get();
        Some(rn)
    }
}
