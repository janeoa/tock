// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use kernel::hil::time::Alarm;
use nrf53::chip::Nrf53DefaultPeripherals;

/// This struct, when initialized, instantiates all peripheral drivers for the nrf5340.
/// If a board wishes to use only a subset of these peripherals, this
/// should not be used or imported, and a modified version should be
/// constructed manually in main.rs.
//create all base nrf53 peripherals
pub struct Nrf5340DefaultPeripherals<'a> {
    pub nrf53: Nrf53DefaultPeripherals<'a>,
    pub ieee802154_radio: crate::ieee802154_radio::Radio<'a>,
    pub usbd: crate::usbd::Usbd<'a>,
    pub gpio_port: crate::gpio::Port<'a, { crate::gpio::NUM_PINS }>,
}

impl<'a> Nrf5340DefaultPeripherals<'a> {
    pub unsafe fn new(
        ieee802154_radio_ack_buf: &'static mut [u8; crate::ieee802154_radio::ACK_BUF_SIZE],
    ) -> Self {
        Self {
            nrf53: Nrf53DefaultPeripherals::new(),
            ieee802154_radio: crate::ieee802154_radio::Radio::new(ieee802154_radio_ack_buf),
            usbd: crate::usbd::Usbd::new(),
            gpio_port: crate::gpio::nrf5340_gpio_create(),
        }
    }
    // Necessary for setting up circular dependencies
    pub fn init(&'static self) {
        self.ieee802154_radio.set_timer_ref(&self.nrf53.timer0);
        self.nrf53.timer0.set_alarm_client(&self.ieee802154_radio);
        self.nrf53.pwr_clk.set_usb_client(&self.usbd);
        self.usbd.set_power_ref(&self.nrf53.pwr_clk);
        kernel::deferred_call::DeferredCallClient::register(&self.ieee802154_radio);
        self.nrf53.init();
    }
}
impl<'a> kernel::platform::chip::InterruptService for Nrf5340DefaultPeripherals<'a> {
    unsafe fn service_interrupt(&self, interrupt: u32) -> bool {
        match interrupt {
            crate::peripheral_interrupts::USBD => self.usbd.handle_interrupt(),
            nrf53::peripheral_interrupts::GPIOTE => self.gpio_port.handle_interrupt(),
            nrf53::peripheral_interrupts::RADIO => {
                match (
                    self.ieee802154_radio.is_enabled(),
                    self.nrf53.ble_radio.is_enabled(),
                ) {
                    (false, false) => (),
                    (true, false) => self.ieee802154_radio.handle_interrupt(),
                    (false, true) => self.nrf53.ble_radio.handle_interrupt(),
                    (true, true) => kernel::debug!(
                        "nRF 802.15.4 and BLE radios cannot be simultaneously enabled!"
                    ),
                }
            }
            _ => return self.nrf53.service_interrupt(interrupt),
        }
        true
    }
}
