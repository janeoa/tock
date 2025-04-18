// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Power management

use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::{
    register_bitfields, register_structs, ReadOnly, ReadWrite, WriteOnly,
};
use kernel::utilities::StaticRef;

const POWER_BASE: StaticRef<PowerRegisters> =
    // unsafe { StaticRef::new(0x40000000 as *const PowerRegisters) };
    unsafe { StaticRef::new(0x50005000 as *const PowerRegisters) };

const RAM_POWER_BASE_VMC: StaticRef<RamPowerRegisters> =
    unsafe { StaticRef::new(0x50081000 as *const RamPowerRegisters) };

const USB_POWER_BASE: StaticRef<USBPowerRegisters> =
    unsafe { StaticRef::new(0x50037000 as *const USBPowerRegisters) };

const REGULATOR_BASE: StaticRef<RegulatorRegisters> =
    unsafe { StaticRef::new(0x50004000 as *const RegulatorRegisters) };

// Note: only the nrf52833+ have 9 banks, but we create all of them to avoid
// gating this code by a feature.
const NUM_RAM_BANKS: usize = 9;

register_structs! {
    RegulatorRegisters {
        (0x000 => _reserved0),
        // Main supply status
        (0x428 => mainregstatus: ReadOnly<u32, MainSupply::Register>),
        (0x42C => @END),
    },

    PowerRegisters {
        (0x000 => _reserved0),
        /// Enable Constant Latency mode
        (0x078 => task_constlat: WriteOnly<u32, Task::Register>),
        /// Enable Low-power mode (variable latency)
        (0x07C => task_lowpwr: WriteOnly<u32, Task::Register>),
        (0x080 => _reserved1),
        /// Power failure warning
        (0x108 => event_pofwarn: ReadWrite<u32, Event::Register>),
        (0x10C => _reserved2),
        /// CPU entered WFI/WFE sleep
        (0x114 => event_sleepenter: ReadWrite<u32, Event::Register>),
        /// CPU exited WFI/WFE sleep
        (0x118 => event_sleepexit: ReadWrite<u32, Event::Register>),
        (0x11C => _reserved3),
        /// Enable interrupt
        (0x304 => power_intenset: ReadWrite<u32, PowerInterrupt::Register>),
        /// Disable interrupt
        (0x308 => power_intenclr: ReadWrite<u32, PowerInterrupt::Register>),
        (0x30C => _reserved4),

        // Reset reason
        // (0x400 => resetreas: ReadWrite<u32, ResetReason::Register>),
        // (0x404 => _reserved5),
        // USB supply status
        // (0x438 => usbregstatus: ReadOnly<u32, UsbRegStatus::Register>),
        // (0x43C => _reserved6),
        // System OFF register
        // (0x500 => systemoff: WriteOnly<u32, Task::Register>),
        // (0x504 => _reserved7),
        // Power failure comparator configuration
        // (0x510 => pofcon: ReadWrite<u32, PowerFailure::Register>),
        // (0x514 => _reserved8),

        /// General purpose retention register
        (0x51C => gpregret: ReadWrite<u32, Byte::Register>),
        (0x520 => @END),

        // General purpose retention register
        // (0x520 => gpregret2: ReadWrite<u32, Byte::Register>),
        // (0x524 => _reserved9),
        // Enable DC/DC converter for REG1 stage
        // (0x578 => dcdcen: ReadWrite<u32, Task::Register>),
        // (0x57C => _reserved10),
        // Enable DC/DC converter for REG0 stage
        // (0x580 => dcdcen0: ReadWrite<u32, Task::Register>),
        // (0x584 => _reserved11),
        // Main supply status
        // (0x640 => mainregstatus: ReadOnly<u32, MainSupply::Register>),
        // (0x644 => _reserved12),
        // RAMx power control registers
        // - Address: 0x900 - 0x980 (<= nRF52832)
        // - Address: 0x900 - 0x990 (>= nRF52833)
        // (0x900 => ram: [RamPowerRegisters; NUM_RAM_BANKS]),
        // (0x990 => @END),
    },

    RamPowerRegisters {
        (0x000 => _reserved0),
        /// RAMn power control register.
        /// The RAM size will vary depending on product variant, and the
        /// RAMn register will only be present if the corresponding RAM AHB
        /// slave is present on the device.
        (0x600 => power: ReadWrite<u32, RamPower::Register>),
        /// RAMn power control set register
        (0x604 => powerset: WriteOnly<u32, RamPower::Register>),
        /// RAMn power control clear register
        (0x608 => powerclr: WriteOnly<u32, RamPower::Register>),
        // (0x00C => _reserved),
        (0x60C => @END),
    },

    USBPowerRegisters {
        (0x000 => _reserved0),
        /// Voltage supply detected on VBUS
        (0x100 => event_usbdetected: ReadWrite<u32, Event::Register>),
        /// Voltage supply removed from VBUS
        (0x104 => event_usbremoved: ReadWrite<u32, Event::Register>),
        /// USB 3.3V supply ready
        (0x108 => event_usbpwrrdy: ReadWrite<u32, Event::Register>),
        (0x10C => _reserved1),
        // PUBLISH_USBDETECTED? PUBLISH_USBREMOVED? PUBLISH_USBPWRRDY? INTEN? INTENSET? INTENCLR?
        /// Enable interrupt
        (0x304 => usb_intenset: ReadWrite<u32, USBInterrupt::Register>),
        /// Disable interrupt
        (0x308 => usb_intenclr: ReadWrite<u32, USBInterrupt::Register>),
        (0x30C => _reserved2),
        /// USB supply status
        (0x400 => usbregstatus: ReadOnly<u32, UsbRegStatus::Register>),
        (0x404 => @END),
    }
}

register_bitfields! [u32,
    /// Start task
    Task [
        ENABLE OFFSET(0) NUMBITS(1)
    ],

    /// Read event
    Event [
        READY OFFSET(0) NUMBITS(1)
    ],

    /// Power management Interrupts
    PowerInterrupt [
        POFWARN OFFSET(2) NUMBITS(1),
        SLEEPENTER OFFSET(5) NUMBITS(1),
        SLEEPEXIT OFFSET(6) NUMBITS(1),
    ],

    USBInterrupt [
        USBDETECTED OFFSET(0) NUMBITS(1),
        USBREMOVED OFFSET(1) NUMBITS(1),
        USBPWRRDY OFFSET(2) NUMBITS(1)
    ],

    ResetReason [
        RESETPIN OFFSET(0) NUMBITS(1) [
            Detected = 1
        ],
        DOG OFFSET(1) NUMBITS(1) [
            Detected = 1
        ],
        SREQ OFFSET(2) NUMBITS(1) [
            Detected = 1
        ],
        LOCKUP OFFSET(3) NUMBITS(1) [
            Detected = 1
        ],
        OFF OFFSET(16) NUMBITS(1) [
            Detected = 1
        ],
        LPCOMP OFFSET(17) NUMBITS(1) [
            Detected = 1
        ],
        DIF OFFSET(18) NUMBITS(1) [
            Detected = 1
        ],
        NFC OFFSET(19) NUMBITS(1) [
            Detected = 1
        ],
        VBUS OFFSET(20) NUMBITS(1) [
            Detected = 1
        ]
    ],

    PowerFailure [
        POF OFFSET(0) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ],
        THRESHOLD OFFSET(1) NUMBITS(4) [
            V17 = 4,
            V18 = 5,
            V19 = 6,
            V20 = 7,
            V21 = 8,
            V22 = 9,
            V23 = 10,
            V24 = 11,
            V25 = 12,
            V26 = 13,
            V27 = 14,
            V28 = 15
        ],
        THRESHOLDVDDH OFFSET(8) NUMBITS(4) [
            V27 = 0,
            V28 = 1,
            V29 = 2,
            V30 = 3,
            V31 = 4,
            V32 = 5,
            V33 = 6,
            V34 = 7,
            V35 = 8,
            V36 = 9,
            V37 = 10,
            V38 = 11,
            V39 = 12,
            V40 = 13,
            V41 = 14,
            V42 = 15
        ]
    ],

    Byte [
        VALUE OFFSET(0) NUMBITS(8)
    ],

    UsbRegStatus [
        VBUSDETECT OFFSET(0) NUMBITS(1),
        OUTPUTRDY OFFSET(1) NUMBITS(1)
    ],

    MainSupply [
        MAINREGSTATUS OFFSET(0) NUMBITS(1) [
            Normal = 0,
            High = 1
        ]
    ],

    RamPower [
        S0POWER OFFSET(0) NUMBITS(1),
        S1POWER OFFSET(1) NUMBITS(1),
        S2POWER OFFSET(2) NUMBITS(1),
        S3POWER OFFSET(3) NUMBITS(1),
        S4POWER OFFSET(4) NUMBITS(1),
        S5POWER OFFSET(5) NUMBITS(1),
        S6POWER OFFSET(6) NUMBITS(1),
        S7POWER OFFSET(7) NUMBITS(1),
        S8POWER OFFSET(8) NUMBITS(1),
        S9POWER OFFSET(9) NUMBITS(1),
        S10POWER OFFSET(10) NUMBITS(1),
        S11POWER OFFSET(11) NUMBITS(1),
        S12POWER OFFSET(12) NUMBITS(1),
        S13POWER OFFSET(13) NUMBITS(1),
        S14POWER OFFSET(14) NUMBITS(1),
        S15POWER OFFSET(15) NUMBITS(1),
        S0RETENTION OFFSET(16) NUMBITS(1),
        S1RETENTION OFFSET(17) NUMBITS(1),
        S2RETENTION OFFSET(18) NUMBITS(1),
        S3RETENTION OFFSET(19) NUMBITS(1),
        S4RETENTION OFFSET(20) NUMBITS(1),
        S5RETENTION OFFSET(21) NUMBITS(1),
        S6RETENTION OFFSET(22) NUMBITS(1),
        S7RETENTION OFFSET(23) NUMBITS(1),
        S8RETENTION OFFSET(24) NUMBITS(1),
        S9RETENTION OFFSET(25) NUMBITS(1),
        S10RETENTION OFFSET(26) NUMBITS(1),
        S11RETENTION OFFSET(27) NUMBITS(1),
        S12RETENTION OFFSET(28) NUMBITS(1),
        S13RETENTION OFFSET(29) NUMBITS(1),
        S14RETENTION OFFSET(30) NUMBITS(1),
        S15RETENTION OFFSET(31) NUMBITS(1)
    ]
];

/// The USB state machine needs to be notified of power events (USB detected, USB
/// removed, USB power ready) in order to be initialized and shut down properly.
/// These events come from the power management registers of this module; that's
/// this has a USB client to notify.
pub struct Power<'a> {
    power_registers: StaticRef<PowerRegisters>,
    usb_registers: StaticRef<USBPowerRegisters>,
    regulator_registers: StaticRef<RegulatorRegisters>,
    /// A client to which to notify USB plug-in/plug-out/power-ready events.
    usb_client: OptionalCell<&'a dyn PowerClient>,
}

pub enum MainVoltage {
    /// Normal voltage mode, when supply voltage is connected to both the VDD and
    /// VDDH pins (so that VDD equals VDDH).
    Normal = 0,
    /// High voltage mode, when supply voltage is only connected to the VDDH pin,
    /// and the VDD pin is not connected to any voltage supply.
    High = 1,
}

pub enum PowerEvent {
    PowerFailure,
    EnterSleep,
    ExitSleep,
    UsbPluggedIn,
    UsbPluggedOut,
    UsbPowerReady,
}

pub trait PowerClient {
    fn handle_power_event(&self, event: PowerEvent);
}

impl<'a> Power<'a> {
    pub const fn new() -> Self {
        Power {
            power_registers: POWER_BASE,
            usb_registers: USB_POWER_BASE,
            regulator_registers: REGULATOR_BASE,
            usb_client: OptionalCell::empty(),
        }
    }

    pub fn set_usb_client(&self, client: &'a dyn PowerClient) {
        self.usb_client.set(client);
    }

    pub fn handle_interrupt(&self) {
        // self.disable_all_interrupts();
        self.disable_all_chip_interrupts();
        self.disable_all_usb_interrupts();

        if self.usb_registers.event_usbdetected.is_set(Event::READY) {
            self.usb_registers
                .event_usbdetected
                .write(Event::READY::CLEAR);
            self.usb_client
                .map(|client| client.handle_power_event(PowerEvent::UsbPluggedIn));
        }

        if self.usb_registers.event_usbremoved.is_set(Event::READY) {
            self.usb_registers
                .event_usbremoved
                .write(Event::READY::CLEAR);
            self.usb_client
                .map(|client| client.handle_power_event(PowerEvent::UsbPluggedOut));
        }

        if self.usb_registers.event_usbpwrrdy.is_set(Event::READY) {
            self.usb_registers
                .event_usbpwrrdy
                .write(Event::READY::CLEAR);
            self.usb_client
                .map(|client| client.handle_power_event(PowerEvent::UsbPowerReady));
        }

        // Clearing unused events
        self.power_registers
            .event_pofwarn
            .write(Event::READY::CLEAR);
        self.power_registers
            .event_sleepenter
            .write(Event::READY::CLEAR);
        self.power_registers
            .event_sleepexit
            .write(Event::READY::CLEAR);

        self.enable_usb_interrupts();
        self.enable_chip_interrups();
    }

    pub fn enable_usb_interrupts(&self) {
        // self.registers.intenset.write(
        // self.power_registers.power_intenset.write(
        //     // Interrupt::USBDETECTED::SET + Interrupt::USBREMOVED::SET + Interrupt::USBPWRRDY::SET,
        //     USBInterrupt::USBDETECTED::SET
        //         + USBInterrupt::USBREMOVED::SET
        //         + USBInterrupt::USBPWRRDY::SET,
        // );
        self.usb_registers.usb_intenset.write(
            USBInterrupt::USBDETECTED::SET
                + USBInterrupt::USBREMOVED::SET
                + USBInterrupt::USBPWRRDY::SET,
        );
    }

    pub fn enable_chip_interrups(&self) {
        self.power_registers.power_intenset.write(
            PowerInterrupt::POFWARN::SET
                + PowerInterrupt::SLEEPENTER::SET
                + PowerInterrupt::SLEEPEXIT::SET,
        );
    }

    pub fn enable_usb_interrupt(&self, intr: u32) {
        // self.registers.intenset.set(intr);
        self.usb_registers.usb_intenset.set(intr);
    }

    pub fn clear_usb_interrupt(&self, intr: u32) {
        // self.registers.intenclr.set(intr);
        self.usb_registers.usb_intenclr.set(intr);
    }

    pub fn disable_all_usb_interrupts(&self) {
        // disable all possible interrupts
        // self.registers.intenclr.set(0xffffffff);
        self.usb_registers.usb_intenclr.set(0xffffffff);
    }

    pub fn enable_chip_interrupt(&self, intr: u32) {
        // self.registers.intenset.set(intr);
        self.power_registers.power_intenset.set(intr);
    }
    pub fn clear_chip_interrupt(&self, intr: u32) {
        // self.registers.intenclr.set(intr);
        self.power_registers.power_intenclr.set(intr);
    }
    pub fn disable_all_chip_interrupts(&self) {
        // disable all possible interrupts
        self.power_registers.power_intenclr.set(0xffffffff);
    }

    pub fn get_main_supply_status(&self) -> MainVoltage {
        match self
            .regulator_registers
            .mainregstatus
            .read_as_enum(MainSupply::MAINREGSTATUS)
        {
            Some(MainSupply::MAINREGSTATUS::Value::Normal) => MainVoltage::Normal,
            Some(MainSupply::MAINREGSTATUS::Value::High) => MainVoltage::High,
            // This case shouldn't happen as the register only holds 1 bit.
            None => unreachable!(),
        }
    }

    pub fn is_vbus_present(&self) -> bool {
        self.usb_registers
            .usbregstatus
            .is_set(UsbRegStatus::VBUSDETECT)
    }

    pub fn is_usb_power_ready(&self) -> bool {
        self.usb_registers
            .usbregstatus
            .is_set(UsbRegStatus::OUTPUTRDY)
    }

    /// Return the contents of the GPREGRET (general purpose retention register)
    /// register.
    ///
    /// This register is a "retention" register because it preserves eight bits
    /// of its state across a soft reset.
    ///
    /// This is used to set a flag before a reset to instruct the bootloader to
    /// stay in the bootloader mode.
    pub fn get_gpregret(&self) -> u8 {
        self.power_registers.gpregret.read(Byte::VALUE) as u8
    }

    /// Set the value of the GPREGRET (general purpose retention register)
    /// register.
    ///
    /// This is used to set a flag before a reset to instruct the bootloader to
    /// stay in the bootloader mode.
    pub fn set_gpregret(&self, val: u8) {
        self.power_registers
            .gpregret
            .write(Byte::VALUE.val(val as u32));
    }
}
