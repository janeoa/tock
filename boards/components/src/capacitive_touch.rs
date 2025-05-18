// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2024.

//! Component for initializing the capacitive touch sensor.
//!
//!
//! Usage:
//! ```rust
//! let touch = components::capacitive_touch::CapacitiveTouchComponent::new(
//!     board_kernel,
//!     capsules_extra::capacitive_touch::DRIVER_NUM,
//!     capacitive_touch_component_helper!(
//!         nrf52::gpio::GPIOPin,
//!         &nrf52::gpio::PORT[pin_num],
//!         &timer
//!     )
//! ).finalize(capacitive_touch_component_static!());
//! ```

use capsules_extra::capacitive_touch::CapacitiveTouchSensor;
use core::mem::MaybeUninit;
use kernel::component::Component;
use kernel::hil::gpio;
use kernel::hil::time::Alarm;

// #[macro_export]
// macro_rules! capacitive_touch_component_helper {
//     ($Pin:ty, $A:expr, $(($P:expr)),+ $(,)?) => {{
//         use kernel::static_init;
//         (&[$($P),+], $A)
//     }};
// }
#[macro_export]
// macro_rules! capacitive_touch_component_static {
//     () => {{
//         use kernel::static_init;
//         static_init!(core::mem::MaybeUninit::uninit())
//     }};
// }

pub struct CapacitiveTouchComponent<'a, A: Alarm<'a>> {
    pins: &'a [&'a dyn gpio::Pin],
    alarm: &'a A,
    kernel: &'static kernel::Kernel,
    driver_num: usize,
}

impl<'a, A: Alarm<'a>> CapacitiveTouchComponent<'a, A> {
    pub fn new(
        kernel: &'static kernel::Kernel,
        driver_num: usize,
        (pins, alarm): (&'a [&'a dyn gpio::Pin], &'a A),
    ) -> Self {
        Self {
            pins,
            alarm,
            kernel,
            driver_num,
        }
    }
}

impl<A: Alarm<'static>> Component for CapacitiveTouchComponent<'static, A> {
    type StaticInput = &'static mut MaybeUninit<CapacitiveTouchSensor<'static, A>>;
    type Output = &'static CapacitiveTouchSensor<'static, A>;

    fn finalize(self, static_buffer: Self::StaticInput) -> Self::Output {
        let touch = CapacitiveTouchSensor::new(self.pins, self.alarm);
        static_buffer.write(touch);
        unsafe { static_buffer.assume_init_ref() }
    }
}
