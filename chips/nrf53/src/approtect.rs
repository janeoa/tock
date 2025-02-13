// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2023.

//! Access port protection
//!
//! <https://infocenter.nordicsemi.com/index.jsp?topic=%2Fps_nrf52840%2Fdif.html&cp=5_0_0_3_7_1&anchor=register.DISABLE>
//!
//! The logic around APPROTECT was changed in newer revisions of the nRF52
//! series chips (Oct 2021) and later which requires more careful disabling of
//! the access port (JTAG), both in the UICR register and in a software written
//! register. This module enables the kernel to disable the protection on boot.
//!
//! Example code to disable the APPROTECT protection in software:
//!
//! ```rust,ignore
//! let approtect = nrf52::approtect::Approtect::new();
//! approtect.sw_disable_approtect();
//! ```

// use crate::ficr;
use kernel::utilities::registers::interfaces::Writeable;
use kernel::utilities::registers::{register_bitfields, register_structs, ReadWrite};
use kernel::utilities::StaticRef;

const APPROTECT_BASE: StaticRef<DebuggerRegisters> =
    // The nrf52 used to have 0x40000000, the nrf53 has 0x50000000
    unsafe { StaticRef::new(0x50000000 as *const DebuggerRegisters) };

register_structs! {
    // CTRL-AP - Control access port
    DebuggerRegisters {
        (0x000 => _system_reset_request),
        (0x004 => _erase_all_request),
        (0x008 => _erase_all_status),
        (0x00C => _approtect_status),
        (0x010 => approtect_disable: ReadWrite<u32, Disable::Register>),
        (0x014 => _reserved0),
        (0x018 => _reserved1),
        (0x01C => _reserved2),
        (0x020 => @END),
        // (0x550 => forceprotect: ReadWrite<u32, Forceprotect::Register>),
        // (0x554 => _reserved1),
        // (0x558 => disable: ReadWrite<u32, Disable::Register>),

    }
}

register_bitfields! [u32,
    Approtect_enable [
        APPROTECT_ENABLE OFFSET(0) NUMBITS(8) [
            FORCE = 0
        ]
    ],
    /// Access port protection
    Disable [
        DISABLE OFFSET(0) NUMBITS(8) [
            APPROTECT_SHARED_VALUE = 0x50FA50FA
        ]
    ]
];

pub struct Approtect {
    registers: StaticRef<DebuggerRegisters>,
}

impl Approtect {
    pub const fn new() -> Approtect {
        Approtect {
            registers: APPROTECT_BASE,
        }
    }

    /// Software disable the Access Port Protection mechanism.
    ///
    /// On newer variants of the nRF52, to enable JTAG, APPROTECT must be
    /// disabled both in the UICR register (hardware) and in this register
    /// (software). For older variants this is just a no-op.
    ///
    /// - <https://devzone.nordicsemi.com/f/nordic-q-a/96590/how-to-disable-approtect-permanently-dfu-is-needed>
    /// - <https://devzone.nordicsemi.com/nordic/nordic-blog/b/blog/posts/working-with-the-nrf52-series-improved-approtect>
    pub fn sw_disable_approtect(&self) {
        // let factory_config = ficr::Ficr::new();
        // I have deleted the checks from the nrf52 because I assume all nrf53 have approtect enabled by default
        self.registers
            .approtect_disable
            .write(Disable::DISABLE::APPROTECT_SHARED_VALUE);
        // const DISABLE_KEY: u32 = 0x50FA50FA; // lets assume the
    }
}
