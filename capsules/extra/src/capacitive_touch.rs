// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

//! Capsule for capacitive touch sensing using GPIO and Timer.

pub const DRIVER_NUM: usize = 0x60000;

use kernel::hil::gpio;
use kernel::hil::time::{Alarm, AlarmClient};
use kernel::utilities::cells::OptionalCell;

pub trait CapacitiveTouchClient {
    /// Called when touch state changes
    /// pin_index: The index of the pin that triggered the event
    fn touch_event(&self, pin_index: usize, is_touched: bool);
}

pub struct CapacitiveTouchSensor<'a, A: Alarm<'a>> {
    /// GPIO pins connected to the capacitive touch sensors
    pins: &'a [&'a dyn gpio::Pin],
    /// Alarm for timing measurements
    alarm: &'a A,
    /// Client to receive touch events
    client: OptionalCell<&'a dyn CapacitiveTouchClient>,
    /// Current pin being measured
    current_pin: usize,
}

impl<'a, A: Alarm<'a>> CapacitiveTouchSensor<'a, A> {
    pub fn new(pins: &'a [&'a dyn gpio::Pin], alarm: &'a A) -> Self {
        Self {
            pins,
            alarm,
            client: OptionalCell::empty(),
            current_pin: 0,
        }
    }

    pub fn set_client(&self, client: &'a dyn CapacitiveTouchClient) {
        self.client.set(client);
    }
}

impl<'a, A: Alarm<'a>> AlarmClient for CapacitiveTouchSensor<'a, A> {
    fn alarm(&self) {
        // Will implement timing logic here later
    }
}
