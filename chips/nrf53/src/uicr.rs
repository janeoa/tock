// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! User information configuration registers
//!
//! Minimal implementation to support activation of the reset button on
//! nRF52-DK.

// TODO: do we need ficr on nrf53?
// use crate::ficr;
// use enum_primitive::cast::FromPrimitive;
// use kernel::debug::debug_println;
// use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
// use kernel::utilities::registers::{register_bitfields, register_structs, ReadWrite};
use kernel::utilities::registers::register_bitfields;
// use kernel::utilities::StaticRef;

// use kernel::utilities::registers::{register_bitfields, register_structs, ReadWrite};

// use crate::gpio::Pin;

// const UICR_BASE: StaticRef<UicrRegisters> =
// //     unsafe { StaticRef::new(0x10001200 as *const UicrRegisters) };
// TODO: Approtect
// register_structs! {
//     // CTRL-AP - Control access port
//     UicrRegisters {
//         (0x000 => approtect: ReadWrite<u32, ApProtect::Register>),
//         (0x004 => _reserved1),
//         (0x00C => extsupply: ReadWrite<u32, ExtSupply::Register>),
//         (0x010 => _reserved2),
//         (0x01C => secure_approtect: ReadWrite<u32, ApProtect::Register>),
//         (0x020 => _reserved3),
//         (0x028 => nfcpins),
//         (0x02C => _reserved4),
//         (0x030 => @END),
//     }
// }

register_bitfields! [u32,
    /// Access port protection
    ApProtect [
        /// Ready event
        ApProtectVals OFFSET(0) NUMBITS(32) [
            /// Enable
            UNPROTECTED = 0xFFFFFFFF,
            /// Disable for later nRF52 variants
            PROTECTED   = 0x00000000,
            /// Disable
            CUSTOM      = 0x69696969,
        ]
    ],
    /// Setting of pins dedicated to NFC functionality: NFC antenna or GPIO
    NfcPins [
        /// Setting pins dedicated to NFC functionality
        PROTECT OFFSET(0) NUMBITS(1) [
            /// Operation as GPIO pins. Same protection as normal GPIO pins
            DISABLED = 0,
            /// Operation as NFC antenna pins. Configures the protection for
            /// NFC operation
            NFC = 1
        ]
    ],
    /// Enable external circuitry to be supplied from VDD pin
    ExtSupply [
        /// Enable external circuitry to be supplied from VDD pin
        EXTSUPPLY OFFSET(0) NUMBITS(1) [
            /// No current can be drawn from the VDD pin
            DISABLED = 0,
            /// It is allowed to supply external circuitry from the VDD pin
            ENABLED = 1
        ]
    ],
    /// GPIO reference voltage / external output supply voltage
    RegOut [
        /// Output voltage from REG0 regulator stage
        VOUT OFFSET(0) NUMBITS(3) [
            V1_8 = 0,
            V2_1 = 1,
            V2_4 = 2,
            V2_7 = 3,
            V3_0 = 4,
            V3_3 = 5,
            DEFAULT = 7
        ]
    ]
];

pub struct Uicr {
    // registers: StaticRef<UicrRegisters>,
}

#[derive(Copy, Clone, PartialEq)]
/// Output voltage from REG0 regulator stage.
/// The value is board dependent (e.g. the nRF52840dk board uses 1.8V
/// whereas the nRF52840-Dongle requires 3.0V to light its LEDs).
/// When a chip is out of the factory or fully erased, the default value (7)
/// will output 1.8V.
pub enum Regulator0Output {
    V1_8 = 0,
    V2_1 = 1,
    V2_4 = 2,
    V2_7 = 3,
    V3_0 = 4,
    V3_3 = 5,
    DEFAULT = 7,
}

impl From<u32> for Regulator0Output {
    fn from(val: u32) -> Self {
        match val & 7 {
            0 => Regulator0Output::V1_8,
            1 => Regulator0Output::V2_1,
            2 => Regulator0Output::V2_4,
            3 => Regulator0Output::V2_7,
            4 => Regulator0Output::V3_0,
            5 => Regulator0Output::V3_3,
            7 => Regulator0Output::DEFAULT,
            _ => Regulator0Output::DEFAULT, // Invalid value, fall back to DEFAULT
        }
    }
}

impl Uicr {
    pub const fn new() -> Uicr {
        Uicr {
            // registers: UICR_BASE,
        }
    }

    pub fn is_ap_protect_enabled(&self) -> bool {
        // We need to understand the variant of this nRF52 chip to correctly
        // implement this function. Newer versions use a different value to
        // indicate disabled.
        // let factory_config = ficr::Ficr::new();
        // let disabled_val = if factory_config.has_updated_approtect_logic() {
        //     ApProtect::PALL::HWDISABLE
        // } else {
        //     ApProtect::PALL::DISABLED
        // };

        // // Here we compare to the correct DISABLED value because any other value
        // // should enable the protection.
        // !self.registers.approtect.matches_all(disabled_val)
        let reg_value = unsafe { core::ptr::read_volatile(0x5000_600C as *const u32) };
        // debug!("APPROTECT: 0x{:08x}", reg_value);
        reg_value != 0x00000000
        // debug_println(");
    }

    // pub fn set_ap_protect(&self) {
    //     self.registers.approtect.write(ApProtect::PALL::ENABLED);
    // }

    /// Disable the access port protection in the UICR register. This is stored
    /// in flash and is persistent. This behavior can also be accomplished
    /// outside of tock by running `nrfjprog --recover`.
    pub fn disable_ap_protect(&self) {
        // We need to understand the variant of this nRF52 chip to correctly
        // implement this function.
        // let factory_config = ficr::Ficr::new();

        // All other revisions just use normal disable.
        // self.registers
        //     .approtect
        //     .write(ApProtect::ApProtectVals::CUSTOM);
    }
}
