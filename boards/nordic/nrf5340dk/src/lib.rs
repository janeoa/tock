// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Tock kernel for the Nordic Semiconductor nRF52840 development kit (DK).
//!
//! It is based on nRF52840 SoC (Cortex M4 core with a BLE transceiver) with
//! many exported I/O and peripherals.
//!
//! Pin Configuration
//! -------------------
//!
//! ### `GPIO`
//!
//! | #  | Pin   | Ix | Header | Arduino |
//! |----|-------|----|--------|---------|
//! | 0  | P1.01 | 33 | P3 1   | D0      |
//! | 1  | P1.02 | 34 | P3 2   | D1      |
//! | 2  | P1.03 | 35 | P3 3   | D2      |
//! | 3  | P1.04 | 36 | P3 4   | D3      |
//! | 4  | P1.05 | 37 | P3 5   | D4      |
//! | 5  | P1.06 | 38 | P3 6   | D5      |
//! | 6  | P1.07 | 39 | P3 7   | D6      |
//! | 7  | P1.08 | 40 | P3 8   | D7      |
//! | 8  | P1.10 | 42 | P4 1   | D8      |
//! | 9  | P1.11 | 43 | P4 2   | D9      |
//! | 10 | P1.12 | 44 | P4 3   | D10     |
//! | 11 | P1.13 | 45 | P4 4   | D11     |
//! | 12 | P1.14 | 46 | P4 5   | D12     |
//! | 13 | P1.15 | 47 | P4 6   | D13     |
//! | 14 | P0.26 | 26 | P4 9   | D14     |
//! | 15 | P0.27 | 27 | P4 10  | D15     |
//!
//! ### `GPIO` / Analog Inputs
//!
//! | #  | Pin        | Header | Arduino |
//! |----|------------|--------|---------|
//! | 16 | P0.03 AIN1 | P2 1   | A0      |
//! | 17 | P0.04 AIN2 | P2 2   | A1      |
//! | 18 | P0.28 AIN4 | P2 3   | A2      |
//! | 19 | P0.29 AIN5 | P2 4   | A3      |
//! | 20 | P0.30 AIN6 | P2 5   | A4      |
//! | 21 | P0.31 AIN7 | P2 6   | A5      |
//! | 22 | P0.02 AIN0 | P4 8   | AVDD    |
//!
//! ### Onboard Functions
//!
//! | Pin   | Header | Function |
//! |-------|--------|----------|
//! | P0.05 | P6 3   | UART RTS |
//! | P0.06 | P6 4   | UART TXD |
//! | P0.07 | P6 5   | UART CTS |
//! | P0.08 | P6 6   | UART RXT |
//! | P0.11 | P24 1  | Button 1 |
//! | P0.12 | P24 2  | Button 2 |
//! | P0.13 | P24 3  | LED 1    |
//! | P0.14 | P24 4  | LED 2    |
//! | P0.15 | P24 5  | LED 3    |
//! | P0.16 | P24 6  | LED 4    |
//! | P0.18 | P24 8  | Reset    |
//! | P0.19 | P24 9  | SPI CLK  |
//! | P0.20 | P24 10 | SPI MOSI |
//! | P0.21 | P24 11 | SPI MISO |
//! | P0.22 | P24 12 | SPI CS   |
//! | P0.24 | P24 14 | Button 3 |
//! | P0.25 | P24 15 | Button 4 |
//! | P0.26 | P24 16 | I2C SDA  |
//! | P0.27 | P24 17 | I2C SCL  |

#![no_std]
#![deny(missing_docs)]

use core::ptr::addr_of;

use capsules_core::button;
use capsules_core::capacitive_touch::CapacitiveTouchSensor;
use capsules_core::virtualizers::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
// use capsules_extra::net::ieee802154::MacAddress;
// use capsules_extra::net::ipv6::ip_utils::IPAddr;
// use crate::usb_ctap;
use capsules_extra::adc_entropy;
use capsules_extra::usb_ctap;
use kernel::component::Component;
use kernel::hil::adc::Adc;
use kernel::hil::gpio;

use components::button_component_helper;
use kernel::deferred_call::DeferredCallClient;
use kernel::hil::gpio::InterruptWithValue;
use kernel::hil::led::LedLow;
use kernel::hil::time::Alarm;
use kernel::hil::time::ConvertTicks;
use kernel::hil::time::Counter;
#[allow(unused_imports)]
use kernel::hil::usb::Client;
use kernel::platform::{KernelResources, SyscallDriverLookup};
use kernel::scheduler::round_robin::RoundRobinSched;
use kernel::StorageLocation;
use kernel::StorageType;
#[allow(unused_imports)]
use kernel::{capabilities, create_capability, debug, debug_gpio, debug_verbose, static_init};
use nrf5340::gpio::Pin;
use nrf5340::interrupt_service::Nrf5340DefaultPeripherals;
use nrf5340::nvmc;
use nrf5340::rtc::Rtc;
use nrf53_components::{UartChannel, UartPins};

const VENDOR_ID: u16 = 0x1915; // Nordic Semiconductor
const PRODUCT_ID: u16 = 0x521f; // nRF5340 Dongle (PCA10059)
const THRESHOLD: u32 = 20;

static STRINGS: &'static [&'static str] = &[
    // Manufacturer
    "Nordic Semiconductor ASA",
    // Product
    "OpenSK",
    // Serial number
    "v1.0",
    // Interface description + main HID string
    "FIDO2",
    // vendor HID string
    "Vendor HID",
];

// The nRF52840DK LEDs (see back of board)
// const LED1_PIN: Pin = Pin::P0_28;
// const LED2_PIN: Pin = Pin::P0_29;
// const LED3_PIN: Pin = Pin::P0_30;
// const LED4_PIN: Pin = Pin::P0_31;

// The nrf53 demo board LEDs
const LEDG_PIN: Pin = Pin::P0_24;
const LEDR_PIN: Pin = Pin::P0_26;
const LEDB_PIN: Pin = Pin::P1_08;

// Capacitive touch pins
const CAP_TOUCH1_PIN: Pin = Pin::P0_05; // Choose an appropriate pin
const CAP_TOUCH2_PIN: Pin = Pin::P0_06; // Choose an appropriate pin

// The nRF52840DK buttons (see back of board)
const BUTTON1_PIN: Pin = Pin::P0_23;
// const BUTTON2_PIN: Pin = Pin::P0_24;
// const BUTTON3_PIN: Pin = Pin::P0_08;
// const BUTTON4_PIN: Pin = Pin::P0_09;
// const BUTTON_RST_PIN: Pin = Pin::P0_18;

// const UART_RTS: Option<Pin> = Some(Pin::P0_05);
// const UART_TXD: Pin = Pin::P0_06;
// const UART_CTS: Option<Pin> = Some(Pin::P0_07);
// const UART_RXD: Pin = Pin::P0_08;
const UART_TXD: Pin = Pin::P1_01;
const UART_RXD: Pin = Pin::P1_00;
const UART_RTS: Option<Pin> = Some(Pin::P0_11);
const UART_CTS: Option<Pin> = Some(Pin::P0_10);

/*
SPI_CS: P0.24
SPI_CLK: P0.19
SPI_MOSI: P0.20
SPI_MISO: P0.21 */
const SPI_CS: Pin = Pin::P0_24;
const SPI_CLK: Pin = Pin::P0_19;
const SPI_MOSI: Pin = Pin::P0_20;
const SPI_MISO: Pin = Pin::P0_21;

// External flash pins removed - using internal flash for TicKV
// const SPI_MX25R6435F_CHIP_SELECT: Pin = Pin::P0_17;
// const SPI_MX25R6435F_WRITE_PROTECT_PIN: Pin = Pin::P0_22;
// const SPI_MX25R6435F_HOLD_PIN: Pin = Pin::P0_23;

// /// I2C pins
// const I2C_SDA_PIN: Pin = Pin::P0_26;
// const I2C_SCL_PIN: Pin = Pin::P0_27;

// Constants related to the configuration of the 15.4 network stack
// const PAN_ID: u16 = 0xABCD;
// const DST_MAC_ADDR: capsules_extra::net::ieee802154::MacAddress =
//     capsules_extra::net::ieee802154::MacAddress::Short(49138);
// const DEFAULT_CTX_PREFIX_LEN: u8 = 8; //Length of context for 6LoWPAN compression
// const DEFAULT_CTX_PREFIX: [u8; 16] = [0x0_u8; 16]; //Context for 6LoWPAN Compression

/// Debug Writer
pub mod io;

// Whether to use UART debugging or Segger RTT (USB) debugging.
// - Set to false to use UART.
// - Set to true to use Segger RTT over USB.
const USB_DEBUGGING: bool = false;

/// This platform's chip type:
pub type Chip = nrf5340::chip::NRF53<'static, Nrf5340DefaultPeripherals<'static>>;

/// Number of concurrent processes this platform supports.
pub const NUM_PROCS: usize = 8;

/// Process array of this platform.
pub static mut PROCESSES: [Option<&'static dyn kernel::process::Process>; NUM_PROCS] =
    [None; NUM_PROCS];

static mut CHIP: Option<&'static nrf5340::chip::NRF53<Nrf5340DefaultPeripherals>> = None;
static mut PROCESS_PRINTER: Option<&'static capsules_system::process_printer::ProcessPrinterText> =
    None;

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x2000] = [0; 0x2000];
/// Flash buffer for the custom nvmc driver
static mut APP_FLASH_BUFFER: [u8; 0x1000] = [0; 0x1000];

static mut STORAGE_LOCATIONS: [StorageLocation; 2] = [
    // We implement NUM_PAGES = 20 as 16 + 4 to satisfy the MPU.
    StorageLocation {
        address: 0xC0000,
        size: 0x10000, // 16 pages
        storage_type: StorageType::Store,
    },
    StorageLocation {
        address: 0xD0000,
        size: 0x4000, // 4 pages
        storage_type: StorageType::Store,
    },
];
//------------------------------------------------------------------------------
// SYSCALL DRIVER TYPE DEFINITIONS
//------------------------------------------------------------------------------

type AlarmDriver = components::alarm::AlarmDriverComponentType<nrf5340::rtc::Rtc<'static>>;
type RngDriver =
    components::rng::RngComponentType<adc_entropy::AdcEntropy<'static, nrf5340::adc::Adc<'static>>>;

// TicKV - Using internal flash instead of external flash
type InternalFlash = nrf5340::nvmc::Nvmc;
const TICKV_PAGE_SIZE: usize =
    core::mem::size_of::<<InternalFlash as kernel::hil::flash::Flash>::Page>();
type Siphasher24 = components::siphash::Siphasher24ComponentType;
type TicKVDedicatedFlash = components::tickv::TicKVDedicatedFlashComponentType<
    InternalFlash,
    Siphasher24,
    TICKV_PAGE_SIZE,
>;
type TicKVKVStore = components::kv::TicKVKVStoreComponentType<
    TicKVDedicatedFlash,
    capsules_extra::tickv::TicKVKeyType,
>;
type KVStorePermissions = components::kv::KVStorePermissionsComponentType<TicKVKVStore>;
type VirtualKVPermissions = components::kv::VirtualKVPermissionsComponentType<KVStorePermissions>;
type KVDriver = components::kv::KVDriverComponentType<VirtualKVPermissions>;

// Temperature
// type TemperatureDriver =
//     components::temperature::TemperatureComponentType<nrf5340::temperature::Temp<'static>>;

// IEEE 802.15.4
// type Ieee802154MacDevice = components::ieee802154::Ieee802154ComponentMacDeviceType<
//     nrf5340::ieee802154_radio::Radio<'static>,
//     nrf5340::aes::AesECB<'static>,
// >;
/// Userspace 802.15.4 driver with in-kernel packet framing and MAC layer.
// pub type Ieee802154Driver = components::ieee802154::Ieee802154ComponentType<
//     nrf5340::ieee802154_radio::Radio<'static>,
//     nrf5340::aes::AesECB<'static>,
// >;

// EUI64
/// Userspace EUI64 driver.
pub type Eui64Driver = components::eui64::Eui64ComponentType;

/// Supported drivers by the platform
pub struct Platform {
    // ble_radio: &'static capsules_extra::ble_advertising_driver::BLE<
    // 'static,
    // nrf5340::ble_radio::Radio<'static>,
    // VirtualMuxAlarm<'static, nrf5340::rtc::Rtc<'static>>,
    // >,
    // button: &'static capsules_core::button::Button<'static, nrf5340::gpio::GPIOPin<'static>>,
    pconsole: &'static capsules_core::process_console::ProcessConsole<
        'static,
        { capsules_core::process_console::DEFAULT_COMMAND_HISTORY_LEN },
        VirtualMuxAlarm<'static, nrf5340::rtc::Rtc<'static>>,
        components::process_console::Capability,
    >,
    console: &'static capsules_core::console::Console<'static>,
    // gpio: &'static capsules_core::gpio::GPIO<'static, nrf5340::gpio::GPIOPin<'static>>,
    led: &'static capsules_core::led::LedDriver<
        'static,
        kernel::hil::led::LedLow<'static, nrf5340::gpio::GPIOPin<'static>>,
        // 4,
        3,
    >,
    rng: &'static RngDriver,
    // adc: &'static capsules_core::adc::AdcDedicated<'static, nrf5340::adc::Adc<'static>>,
    // temp: &'static TemperatureDriver,
    /// The IPC driver.
    pub ipc: kernel::ipc::IPC<{ NUM_PROCS as u8 }>,
    // analog_comparator: &'static capsules_extra::analog_comparator::AnalogComparator<
    //     'static,
    //     nrf5340::acomp::Comparator<'static>,
    // >,
    alarm: &'static AlarmDriver,
    // i2c_master_slave: &'static capsules_core::i2c_master_slave_driver::I2CMasterSlaveDriver<
    //     'static,
    //     nrf5340::i2c::TWI<'static>,
    // >,
    // spi_controller: &'static capsules_core::spi_controller::Spi<
    //     'static,
    //     capsules_core::virtualizers::virtual_spi::VirtualSpiMasterDevice<
    //         'static,
    //         nrf5340::spi::SPIM<'static>,
    //     >,
    // >,
    // kv_driver: &'static KVDriver,
    nvmc: &'static nrf5340::nvmc::SyscallDriver,
    // usb: &'static components::usb_ctap::UsbCtapComponent<'static>,
    // usb: &'static capsules::usb::usb_ctap::CtapUsbSyscallDriver<
    usb: &'static capsules_extra::usb::usb_ctap::CtapUsbSyscallDriver<
        'static,
        'static,
        nrf5340::usbd::Usbd<'static>,
    >,
    scheduler: &'static RoundRobinSched<'static>,
    systick: cortexm33::systick::SysTick,
    capacitive_touch: &'static capsules_core::button::Button<
        'static,
        capsules_core::capacitive_touch::CapacitiveTouchSensor<
            'static,
            VirtualMuxAlarm<'static, nrf5340::rtc::Rtc<'static>>,
        >,
    >,
}

impl SyscallDriverLookup for Platform {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::syscall::SyscallDriver>) -> R,
    {
        match driver_num {
            capsules_core::console::DRIVER_NUM => f(Some(self.console)),
            // capsules_core::gpio::DRIVER_NUM => f(Some(self.gpio)),
            capsules_core::alarm::DRIVER_NUM => f(Some(self.alarm)),
            capsules_core::led::DRIVER_NUM => f(Some(self.led)),
            // capsules_extra::usb_ctap::DRIVER_NUM => f(Some(self.usb)),
            capsules_extra::usb::usb_ctap::DRIVER_NUM => f(Some(self.usb)),
            capsules_core::button::DRIVER_NUM => f(Some(self.capacitive_touch)),
            capsules_core::rng::DRIVER_NUM => f(Some(self.rng)),
            // capsules_core::adc::DRIVER_NUM => f(Some(self.adc)),
            // capsules_extra::ble_advertising_driver::DRIVER_NUM => f(Some(self.ble_radio)),
            // capsules_extra::temperature::DRIVER_NUM => f(Some(self.temp)),
            // capsules_extra::analog_comparator::DRIVER_NUM => f(Some(self.analog_comparator)),
            kernel::ipc::DRIVER_NUM => f(Some(&self.ipc)),
            // capsules_core::i2c_master_slave_driver::DRIVER_NUM => f(Some(self.i2c_master_slave)),
            // capsules_core::spi_controller::DRIVER_NUM => f(Some(self.spi_controller)),
            // capsules_extra::kv_driver::DRIVER_NUM => f(Some(self.kv_driver)),
            nrf5340::nvmc::DRIVER_NUM => f(Some(self.nvmc)),
            _ => f(None),
        }
    }
}

impl KernelResources<Chip> for Platform {
    type SyscallDriverLookup = Self;
    type SyscallFilter = ();
    type ProcessFault = ();
    type Scheduler = RoundRobinSched<'static>;
    type SchedulerTimer = cortexm33::systick::SysTick;
    type WatchDog = ();
    type ContextSwitchCallback = ();

    fn syscall_driver_lookup(&self) -> &Self::SyscallDriverLookup {
        self
    }
    fn syscall_filter(&self) -> &Self::SyscallFilter {
        &()
    }
    fn process_fault(&self) -> &Self::ProcessFault {
        &()
    }
    fn scheduler(&self) -> &Self::Scheduler {
        self.scheduler
    }
    fn scheduler_timer(&self) -> &Self::SchedulerTimer {
        &self.systick
    }
    fn watchdog(&self) -> &Self::WatchDog {
        &()
    }
    fn context_switch_callback(&self) -> &Self::ContextSwitchCallback {
        &()
    }
}

/// Create the capsules needed for the in-kernel UDP and 15.4 stack.
// pub unsafe fn ieee802154_udp(
//     board_kernel: &'static kernel::Kernel,
//     nrf5340_peripherals: &'static Nrf5340DefaultPeripherals<'static>,
//     mux_alarm: &'static MuxAlarm<nrf5340::rtc::Rtc>,
// ) -> (
//     &'static Eui64Driver,
//     &'static Ieee802154Driver,
//     &'static capsules_extra::net::udp::UDPDriver<'static>,
// ) {
//     //--------------------------------------------------------------------------
//     // AES
//     //--------------------------------------------------------------------------

//     let aes_mux =
//         components::ieee802154::MuxAes128ccmComponent::new(&nrf5340_peripherals.nrf53.ecb)
//             .finalize(components::mux_aes128ccm_component_static!(
//                 nrf5340::aes::AesECB
//             ));

//     //--------------------------------------------------------------------------
//     // 802.15.4
//     //--------------------------------------------------------------------------

//     // let device_id = nrf5340::ficr::FICR_INSTANCE.id();
//     let device_id = (*addr_of!(nrf5340::ficr::FICR_INSTANCE)).id();
//     let device_id_bottom_16: u16 = u16::from_le_bytes([device_id[0], device_id[1]]);

//     let eui64_driver = components::eui64::Eui64Component::new(u64::from_le_bytes(device_id))
//         .finalize(components::eui64_component_static!());

// let (ieee802154_driver, mux_mac) = components::ieee802154::Ieee802154Component::new(
//     board_kernel,
//     capsules_extra::ieee802154::DRIVER_NUM,
//     &nrf5340_peripherals.ieee802154_radio,
//     aes_mux,
//     PAN_ID,
//     device_id_bottom_16,
//     device_id,
// )
// .finalize(components::ieee802154_component_static!(
//     nrf5340::ieee802154_radio::Radio,
//     nrf5340::aes::AesECB<'static>
// ));

//--------------------------------------------------------------------------
// UDP
//--------------------------------------------------------------------------

//     let local_ip_ifaces = static_init!(
//         [IPAddr; 3],
//         [
//             IPAddr::generate_from_mac(capsules_extra::net::ieee802154::MacAddress::Long(device_id)),
//             IPAddr([
//                 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
//                 0x1e, 0x1f,
//             ]),
//             IPAddr::generate_from_mac(capsules_extra::net::ieee802154::MacAddress::Short(
//                 device_id_bottom_16
//             )),
//         ]
//     );

//     let (udp_send_mux, udp_recv_mux, udp_port_table) = components::udp_mux::UDPMuxComponent::new(
//         mux_mac,
//         DEFAULT_CTX_PREFIX_LEN,
//         DEFAULT_CTX_PREFIX,
//         DST_MAC_ADDR,
//         MacAddress::Long(device_id),
//         local_ip_ifaces,
//         mux_alarm,
//     )
//     .finalize(components::udp_mux_component_static!(
//         nrf5340::rtc::Rtc,
//         Ieee802154MacDevice
//     ));

//     // UDP driver initialization happens here
//     let udp_driver = components::udp_driver::UDPDriverComponent::new(
//         board_kernel,
//         capsules_extra::net::udp::driver::DRIVER_NUM,
//         udp_send_mux,
//         udp_recv_mux,
//         udp_port_table,
//         local_ip_ifaces,
//     )
//     .finalize(components::udp_driver_component_static!(nrf5340::rtc::Rtc));

//     (eui64_driver, ieee802154_driver, udp_driver)
// }

/// This is in a separate, inline(never) function so that its stack frame is
/// removed when this function returns. Otherwise, the stack space used for
/// these static_inits is wasted.
#[inline(never)]
pub unsafe fn start() -> (
    &'static kernel::Kernel,
    Platform,
    &'static Chip,
    &'static Nrf5340DefaultPeripherals<'static>,
    &'static MuxAlarm<'static, nrf5340::rtc::Rtc<'static>>,
) {
    //--------------------------------------------------------------------------
    // INITIAL SETUP
    //--------------------------------------------------------------------------

    // Apply errata fixes and enable interrupts.
    nrf5340::init();

    // Set up peripheral drivers. Called in separate function to reduce stack
    // usage.
    // let ieee802154_ack_buf = static_init!(
    //     [u8; nrf5340::ieee802154_radio::ACK_BUF_SIZE],
    //     [0; nrf5340::ieee802154_radio::ACK_BUF_SIZE]
    // );
    // Initialize chip peripheral drivers
    let nrf5340_peripherals = static_init!(
        Nrf5340DefaultPeripherals,
        // Nrf5340DefaultPeripherals::new(ieee802154_ack_buf)
        Nrf5340DefaultPeripherals::new()
    );

    // Set up circular peripheral dependencies.
    nrf5340_peripherals.init();
    let base_peripherals = &nrf5340_peripherals.nrf53;

    // Configure kernel debug GPIOs as early as possible.
    kernel::debug::assign_gpios(
        // Some(&nrf5340_peripherals.gpio_port[LED1_PIN]),
        // Some(&nrf5340_peripherals.gpio_port[LED2_PIN]),
        // Some(&nrf5340_peripherals.gpio_port[LED3_PIN]),
        Some(&nrf5340_peripherals.gpio_port[LEDR_PIN]),
        Some(&nrf5340_peripherals.gpio_port[LEDG_PIN]),
        Some(&nrf5340_peripherals.gpio_port[LEDB_PIN]),
    );

    // Choose the channel for serial output. This board can be configured to use
    // either the Segger RTT channel or via UART with traditional TX/RX GPIO
    // pins.
    let uart_channel = if USB_DEBUGGING {
        // Initialize early so any panic beyond this point can use the RTT
        // memory object.
        let mut rtt_memory_refs = components::segger_rtt::SeggerRttMemoryComponent::new()
            .finalize(components::segger_rtt_memory_component_static!());

        // XXX: This is inherently unsafe as it aliases the mutable reference to
        // rtt_memory. This aliases reference is only used inside a panic
        // handler, which should be OK, but maybe we should use a const
        // reference to rtt_memory and leverage interior mutability instead.
        self::io::set_rtt_memory(&*rtt_memory_refs.get_rtt_memory_ptr());

        UartChannel::Rtt(rtt_memory_refs)
    } else {
        UartChannel::Pins(UartPins::new(UART_RTS, UART_TXD, UART_CTS, UART_RXD))
    };

    // Setup space to store the core kernel data structure.

    let board_kernel = static_init!(
        kernel::Kernel,
        kernel::Kernel::new_with_storage(&*addr_of!(PROCESSES), &*addr_of!(STORAGE_LOCATIONS))
    );
    // Create (and save for panic debugging) a chip object to setup low-level
    // resources (e.g. MPU, systick).
    let chip = static_init!(Chip, nrf5340::chip::NRF53::new(nrf5340_peripherals));
    CHIP = Some(chip);

    // Do nRF configuration and setup. This is shared code with other nRF-based
    // platforms.
    nrf53_components::startup::NrfStartupComponent::new(
        // false,
        // THIS IS NOT RESET PIN
        // BUTTON1_PIN,
        // nrf5340::uicr::Regulator0Output::DEFAULT,
        &base_peripherals.nvmc,
    )
    .finalize(());

    //--------------------------------------------------------------------------
    // CAPABILITIES
    //--------------------------------------------------------------------------

    // Create capabilities that the board needs to call certain protected kernel
    // functions.
    let memory_allocation_capability = create_capability!(capabilities::MemoryAllocationCapability);
    // let gpio_port = &nrf5340_peripherals.gpio_port;

    //--------------------------------------------------------------------------
    // GPIO
    //--------------------------------------------------------------------------

    // Expose the D0-D13 Arduino GPIO pins to userspace.
    // let gpio = components::gpio::GpioComponent::new(
    //     board_kernel,
    //     capsules_core::gpio::DRIVER_NUM,
    //     components::gpio_component_helper!(
    //         nrf5340::gpio::GPIOPin,
    //         0 => &nrf5340_peripherals.gpio_port[CAP_TOUCH1_PIN],
    //         // 1 => &nrf5340_peripherals.gpio_port[Pin::P1_02],
    //         // 2 => &nrf5340_peripherals.gpio_port[Pin::P1_03],
    //         // 3 => &nrf5340_peripherals.gpio_port[Pin::P1_04],
    //         // 4 => &nrf5340_peripherals.gpio_port[Pin::P1_05],
    //         // 5 => &nrf5340_peripherals.gpio_port[Pin::P1_06],
    //         // 6 => &nrf5340_peripherals.gpio_port[Pin::P1_07],
    //         // 7 => &nrf5340_peripherals.gpio_port[Pin::P1_08],
    //         // Avoid exposing the I2C pins to userspace, as these are used in
    //         // some tutorials (e.g., `nrf5340dk-thread-tutorial`).
    //         //
    //         // In the future we might want to make this configurable.
    //         //
    //         // 8 => &nrf5340_peripherals.gpio_port[Pin::P1_10],
    //         // 9 => &nrf5340_peripherals.gpio_port[Pin::P1_11],
    //         // 10 => &nrf5340_peripherals.gpio_port[Pin::P1_12],
    //         // 11 => &nrf5340_peripherals.gpio_port[Pin::P1_13],
    //         // 12 => &nrf5340_peripherals.gpio_port[Pin::P1_14],
    //         // 13 => &nrf5340_peripherals.gpio_port[Pin::P1_15],
    //     ),
    // )
    // .finalize(components::gpio_component_static!(nrf5340::gpio::GPIOPin));

    //--------------------------------------------------------------------------
    // BUTTONS
    //--------------------------------------------------------------------------
    // let button = components::button::ButtonComponent::new(
    //     board_kernel,
    //     capsules_core::button::DRIVER_NUM,
    //     components::button_component_helper!(
    //         nrf5340::gpio::GPIOPin,
    //         (
    //             &nrf5340_peripherals.gpio_port[BUTTON1_PIN],
    //             kernel::hil::gpio::ActivationMode::ActiveLow,
    //             kernel::hil::gpio::FloatingState::PullUp
    //         ),
    //         // (
    //         //     &nrf5340_peripherals.gpio_port[BUTTON2_PIN],
    //         //     kernel::hil::gpio::ActivationMode::ActiveLow,
    //         //     kernel::hil::gpio::FloatingState::PullUp
    //         // ),
    //         // (
    //         //     &nrf5340_peripherals.gpio_port[BUTTON3_PIN],
    //         //     kernel::hil::gpio::ActivationMode::ActiveLow,
    //         //     kernel::hil::gpio::FloatingState::PullUp
    //         // ),
    //         // (
    //         //     &nrf5340_peripherals.gpio_port[BUTTON4_PIN],
    //         //     kernel::hil::gpio::ActivationMode::ActiveLow,
    //         //     kernel::hil::gpio::FloatingState::PullUp
    //         // )
    //     ),
    // )
    // .finalize(components::button_component_static!(nrf5340::gpio::GPIOPin));

    //--------------------------------------------------------------------------
    // LEDs
    //--------------------------------------------------------------------------

    let led = components::led::LedsComponent::new().finalize(components::led_component_static!(
        LedLow<'static, nrf5340::gpio::GPIOPin>,
        // LedLow::new(&nrf5340_peripherals.gpio_port[LED1_PIN]),
        // LedLow::new(&nrf5340_peripherals.gpio_port[LED2_PIN]),
        // LedLow::new(&nrf5340_peripherals.gpio_port[LED3_PIN]),
        // LedLow::new(&nrf5340_peripherals.gpio_port[LED4_PIN]),
        LedLow::new(&nrf5340_peripherals.gpio_port[LEDR_PIN]),
        LedLow::new(&nrf5340_peripherals.gpio_port[LEDG_PIN]),
        LedLow::new(&nrf5340_peripherals.gpio_port[LEDB_PIN]),
    ));

    //--------------------------------------------------------------------------
    // TIMER
    //--------------------------------------------------------------------------

    let rtc = &base_peripherals.rtc;
    let _ = rtc.start();
    let mux_alarm = components::alarm::AlarmMuxComponent::new(rtc)
        .finalize(components::alarm_mux_component_static!(nrf5340::rtc::Rtc));
    let alarm = components::alarm::AlarmDriverComponent::new(
        board_kernel,
        capsules_core::alarm::DRIVER_NUM,
        mux_alarm,
    )
    .finalize(components::alarm_component_static!(nrf5340::rtc::Rtc));

    //--------------------------------------------------------------------------
    // UART & CONSOLE & DEBUG
    //--------------------------------------------------------------------------

    let uart_channel = nrf53_components::UartChannelComponent::new(
        uart_channel,
        mux_alarm,
        &base_peripherals.uarte1,
    )
    .finalize(nrf53_components::uart_channel_component_static!(
        nrf5340::rtc::Rtc
    ));

    // Tool for displaying information about processes.
    let process_printer = components::process_printer::ProcessPrinterTextComponent::new()
        .finalize(components::process_printer_text_component_static!());
    PROCESS_PRINTER = Some(process_printer);

    // Virtualize the UART channel for the console and for kernel debug.
    let uart_mux = components::console::UartMuxComponent::new(uart_channel, 115200)
        .finalize(components::uart_mux_component_static!());

    // Create the process console, an interactive terminal for managing
    // processes.
    let pconsole = components::process_console::ProcessConsoleComponent::new(
        board_kernel,
        uart_mux,
        mux_alarm,
        process_printer,
        Some(cortexm33::support::reset),
    )
    .finalize(components::process_console_component_static!(
        nrf5340::rtc::Rtc<'static>
    ));

    // Setup the serial console for userspace.
    let console = components::console::ConsoleComponent::new(
        board_kernel,
        capsules_core::console::DRIVER_NUM,
        uart_mux,
    )
    .finalize(components::console_component_static!());

    // Create the debugger object that handles calls to `debug!()`.
    components::debug_writer::DebugWriterComponent::new(uart_mux)
        .finalize(components::debug_writer_component_static!());

    //--------------------------------------------------------------------------
    // BLE
    //--------------------------------------------------------------------------

    // let ble_radio = components::ble::BLEComponent::new(
    //     board_kernel,
    //     capsules_extra::ble_advertising_driver::DRIVER_NUM,
    //     &base_peripherals.ble_radio,
    //     mux_alarm,
    // )
    // .finalize(components::ble_component_static!(
    //     nrf5340::rtc::Rtc,
    //     nrf5340::ble_radio::Radio
    // ));

    //--------------------------------------------------------------------------
    // TEMPERATURE (internal)
    //--------------------------------------------------------------------------

    // let temp = components::temperature::TemperatureComponent::new(
    //     board_kernel,
    //     capsules_extra::temperature::DRIVER_NUM,
    //     &base_peripherals.temp,
    // )
    // .finalize(components::temperature_component_static!(
    //     nrf5340::temperature::Temp
    // ));

    //--------------------------------------------------------------------------
    // ADC
    //--------------------------------------------------------------------------

    // let adc_channels = static_init!(
    //     [nrf5340::adc::AdcChannelSetup; 6],
    //     [
    //         nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput1),
    //         nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput2),
    //         nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput4),
    //         nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput5),
    //         nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput6),
    //         nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput7),
    //     ]
    // );
    // let adc = components::adc::AdcDedicatedComponent::new(
    //     &base_peripherals.adc,
    //     adc_channels,
    //     board_kernel,
    //     capsules_core::adc::DRIVER_NUM,
    // )
    // .finalize(components::adc_dedicated_component_static!(
    //     nrf5340::adc::Adc
    // ));

    //--------------------------------------------------------------------------
    // CAPACITIVE TOUCH SENSORS
    //--------------------------------------------------------------------------

    let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);

    // Create a grant with minimal boilerplate
    let nvmc_grant = board_kernel.create_grant(nvmc::DRIVER_NUM, &grant_cap);
    let nvmc_deferred_call = static_init!(
        kernel::deferred_call::DeferredCall,
        kernel::deferred_call::DeferredCall::new()
    );

    // Create the NVMC syscall driver using the existing deferred call
    let nvmc = static_init!(
        nrf5340::nvmc::SyscallDriver,
        nrf5340::nvmc::SyscallDriver::new(
            &base_peripherals.nvmc,
            nvmc_grant,
            nvmc_deferred_call,
            &mut APP_FLASH_BUFFER
        )
    );

    // Register the driver
    kernel::deferred_call::DeferredCallClient::register(nvmc);

    let cap_touch_alarm1 = static_init!(
        VirtualMuxAlarm<'static, nrf5340::rtc::Rtc>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    cap_touch_alarm1.setup();

    let cap_touch_alarm2 = static_init!(
        VirtualMuxAlarm<'static, nrf5340::rtc::Rtc>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    cap_touch_alarm2.setup();

    // Create capacitive touch sensor 1
    let cap_touch1 = static_init!(
        capsules_core::capacitive_touch::CapacitiveTouchSensor<
            'static,
            VirtualMuxAlarm<'static, nrf5340::rtc::Rtc>,
        >,
        capsules_core::capacitive_touch::CapacitiveTouchSensor::new(
            &nrf5340_peripherals.gpio_port[CAP_TOUCH1_PIN],
            cap_touch_alarm1,
            THRESHOLD.into(),                   // Threshold: 20
            cap_touch_alarm1.ticks_from_ms(10), // Scan interval: 100ms
        )
    );
    cap_touch1.set_pin_id(0);
    cap_touch_alarm1.set_alarm_client(cap_touch1);

    let cap_touch2 = static_init!(
        capsules_core::capacitive_touch::CapacitiveTouchSensor<
            'static,
            VirtualMuxAlarm<'static, nrf5340::rtc::Rtc>,
        >,
        capsules_core::capacitive_touch::CapacitiveTouchSensor::new(
            &nrf5340_peripherals.gpio_port[CAP_TOUCH2_PIN],
            cap_touch_alarm2,
            THRESHOLD.into(),                   // Threshold: 20
            cap_touch_alarm2.ticks_from_ms(10), // Scan interval: 100ms
        )
    );
    cap_touch_alarm2.set_alarm_client(cap_touch2);
    cap_touch2.set_pin_id(1);

    cap_touch1.register();
    cap_touch2.register();

    let cap_touch_wrapper = button_component_helper!(
        capsules_core::capacitive_touch::CapacitiveTouchSensor<
            'static,
            VirtualMuxAlarm<'static, nrf5340::rtc::Rtc>,
        >,
        (
            cap_touch1,
            gpio::ActivationMode::ActiveHigh,
            gpio::FloatingState::PullNone
        ),
        (
            cap_touch2,
            gpio::ActivationMode::ActiveHigh,
            gpio::FloatingState::PullNone
        )
    );

    let cap_touch_button = static_init!(
        capsules_core::button::Button<
            'static,
            capsules_core::capacitive_touch::CapacitiveTouchSensor<
                'static,
                VirtualMuxAlarm<'static, nrf5340::rtc::Rtc>,
            >,
        >,
        capsules_core::button::Button::new(
            cap_touch_wrapper,
            board_kernel.create_grant(capsules_core::button::DRIVER_NUM, &grant_cap)
        )
    );

    // Set the button capsule as the client for the capacitive touch sensor
    cap_touch1.set_client(cap_touch_button);
    cap_touch2.set_client(cap_touch_button);

    // Register capacitive touch sensors with the deferred call system

    //--------------------------------------------------------------------------
    // RANDOM NUMBER GENERATOR
    //--------------------------------------------------------------------------

    let adc_entropy_channel = static_init!(
        nrf5340::adc::AdcChannelSetup,
        nrf5340::adc::AdcChannelSetup::new(nrf5340::adc::AdcChannel::AnalogInput0),
    );

    // Initialize adc_entropy with the ADC hardware directly
    let adc_entropy = static_init!(
        adc_entropy::AdcEntropy<'static, nrf5340::adc::Adc>,
        adc_entropy::AdcEntropy::new(&base_peripherals.adc, adc_entropy_channel)
    );

    // Set the ADC client to adc_entropy
    base_peripherals.adc.set_client(adc_entropy);

    // Use adc_entropy as the entropy source for RNG
    let rng = components::rng::RngComponent::new(
        board_kernel,
        capsules_core::rng::DRIVER_NUM,
        adc_entropy,
    )
    .finalize(components::rng_component_static!(
        adc_entropy::AdcEntropy<'static, nrf5340::adc::Adc>
    ));

    //--------------------------------------------------------------------------
    // SPI
    //--------------------------------------------------------------------------

    let mux_spi = components::spi::SpiMuxComponent::new(&base_peripherals.spim0)
        .finalize(components::spi_mux_component_static!(nrf5340::spi::SPIM));

    // // Create the SPI system call capsule.
    // let spi_controller = components::spi::SpiSyscallComponent::new(
    //     board_kernel,
    //     mux_spi,
    //     kernel::hil::spi::cs::IntoChipSelect::<_, kernel::hil::spi::cs::ActiveLow>::into_cs(
    //         &gpio_port[SPI_CS],
    //     ),
    //     capsules_core::spi_controller::DRIVER_NUM,
    // )
    // .finalize(components::spi_syscall_component_static!(
    //     nrf5340::spi::SPIM
    // ));

    // base_peripherals.spim0.configure(
    //     nrf5340::pinmux::Pinmux::new(SPI_MOSI as u32),
    //     nrf5340::pinmux::Pinmux::new(SPI_MISO as u32),
    //     nrf5340::pinmux::Pinmux::new(SPI_CLK as u32),
    // );

    //--------------------------------------------------------------------------
    // INTERNAL FLASH FOR TICKV
    //--------------------------------------------------------------------------

    // Static buffer to use when reading/writing flash for TicKV.
    // let page_buffer = static_init!(
    //     <InternalFlash as kernel::hil::flash::Flash>::Page,
    //     <InternalFlash as kernel::hil::flash::Flash>::Page::default()
    // );

    // // SipHash for creating TicKV hashed keys.
    // let sip_hash = components::siphash::Siphasher24Component::new()
    //     .finalize(components::siphasher24_component_static!());

    // // TicKV with Tock wrapper/interface using internal flash.
    // // We'll use the last 128KB (32 pages * 4KB per page) of internal flash for TicKV storage.
    // // nRF5340 has 1MB of flash, so we start at page 224 (896KB offset) to avoid kernel/app space.
    // let tickv = components::tickv::TicKVDedicatedFlashComponent::new(
    //     sip_hash,
    //     &base_peripherals.nvmc,
    //     224,       // start at page 224 (896KB offset) to avoid kernel/app space
    //     32 * 4096, // 32 pages * 4KB per page = 128KB for TicKV storage
    //     page_buffer,
    // )
    // .finalize(components::tickv_dedicated_flash_component_static!(
    //     InternalFlash,
    //     Siphasher24,
    //     TICKV_PAGE_SIZE,
    // ));

    // KVSystem interface to KV (built on TicKV).
    // let tickv_kv_store = components::kv::TicKVKVStoreComponent::new(tickv).finalize(
    //     components::tickv_kv_store_component_static!(
    //         TicKVDedicatedFlash,
    //         capsules_extra::tickv::TicKVKeyType,
    //     ),
    // );

    // let kv_store_permissions = components::kv::KVStorePermissionsComponent::new(tickv_kv_store)
    //     .finalize(components::kv_store_permissions_component_static!(
    //         TicKVKVStore
    //     ));

    // // Share the KV stack with a mux.
    // let mux_kv = components::kv::KVPermissionsMuxComponent::new(kv_store_permissions).finalize(
    //     components::kv_permissions_mux_component_static!(KVStorePermissions),
    // );

    // // // Create a virtual component for the userspace driver.
    // let virtual_kv_driver = components::kv::VirtualKVPermissionsComponent::new(mux_kv).finalize(
    //     components::virtual_kv_permissions_component_static!(KVStorePermissions),
    // );

    // // Userspace driver for KV.
    // let kv_driver = components::kv::KVDriverComponent::new(
    //     virtual_kv_driver,
    //     board_kernel,
    //     capsules_extra::kv_driver::DRIVER_NUM,
    // )
    // .finalize(components::kv_driver_component_static!(
    //     VirtualKVPermissions
    // ));

    //--------------------------------------------------------------------------
    // I2C CONTROLLER/TARGET
    //--------------------------------------------------------------------------

    // let i2c_master_slave = components::i2c::I2CMasterSlaveDriverComponent::new(
    //     board_kernel,
    //     capsules_core::i2c_master_slave_driver::DRIVER_NUM,
    //     &base_peripherals.twi1,
    // )
    // .finalize(components::i2c_master_slave_component_static!(
    //     nrf5340::i2c::TWI
    // ));

    // base_peripherals.twi1.configure(
    //     nrf5340::pinmux::Pinmux::new(I2C_SCL_PIN as u32),
    //     nrf5340::pinmux::Pinmux::new(I2C_SDA_PIN as u32),
    // );
    // base_peripherals.twi1.set_speed(nrf5340::i2c::Speed::K400);

    //--------------------------------------------------------------------------
    // ANALOG COMPARATOR
    //--------------------------------------------------------------------------

    // // Initialize AC using AIN5 (P0.29) as VIN+ and VIN- as AIN0 (P0.02)
    // // These are hardcoded pin assignments specified in the driver
    // let analog_comparator = components::analog_comparator::AnalogComparatorComponent::new(
    //     &base_peripherals.acomp,
    //     components::analog_comparator_component_helper!(
    //         nrf5340::acomp::Channel,
    //         &*addr_of!(nrf5340::acomp::CHANNEL_AC0)
    //     ),
    //     board_kernel,
    //     capsules_extra::analog_comparator::DRIVER_NUM,
    // )
    // .finalize(components::analog_comparator_component_static!(
    //     nrf5340::acomp::Comparator
    // ));

    //--------------------------------------------------------------------------
    // NRF CLOCK SETUP
    //--------------------------------------------------------------------------

    nrf53_components::NrfClockComponent::new(&base_peripherals.clock).finalize(());

    //--------------------------------------------------------------------------
    // USB EXAMPLES
    //--------------------------------------------------------------------------
    // Uncomment to experiment with this.

    // let usb = components::usb_ctap::UsbCtapComponent::new(
    // let usb = capsules_extra::usb_ctap::UsbCtapComponent::new(
    let usb = components::usb_ctap::UsbCtapComponent::new(
        board_kernel,
        // capsules::usb::usb_ctap::DRIVER_NUM,
        usb_ctap::DRIVER_NUM,
        &nrf5340_peripherals.usbd,
        // capsules_extra::usb::usbc_client::MAX_CTRL_PACKET_SIZE_NRF5340,
        capsules_extra::usb::usbc_client::MAX_CTRL_PACKET_SIZE_NRF52840,
        VENDOR_ID,
        PRODUCT_ID,
        STRINGS,
    )
    .finalize(components::usb_ctap_component_helper!(nrf5340::usbd::Usbd));

    // // Create the strings we include in the USB descriptor.
    // let strings = static_init!(
    //     [&str; 3],
    //     [
    //         "Nordic Semiconductor", // Manufacturer
    //         "nRF52840dk - TockOS",  // Product
    //         "serial0001",           // Serial number
    //     ]
    // );

    // CTAP Example
    //
    // let (ctap, _ctap_driver) = components::ctap::CtapComponent::new(
    //     board_kernel,
    //     capsules_extra::ctap::DRIVER_NUM,
    //     &nrf5340_peripherals.usbd,
    //     0x1915, // Nordic Semiconductor
    //     0x503a, // lowRISC generic FS USB
    //     strings,
    // )
    // .finalize(components::ctap_component_static!(nrf5340::usbd::Usbd));

    // ctap.enable();
    // ctap.attach();

    // // Keyboard HID Example
    // type UsbHw = nrf5340::usbd::Usbd<'static>;
    // let usb_device = &nrf5340_peripherals.usbd;

    // let (keyboard_hid, keyboard_hid_driver) = components::keyboard_hid::KeyboardHidComponent::new(
    //     board_kernel,
    //     capsules_core::driver::NUM::KeyboardHid as usize,
    //     usb_device,
    //     0x1915, // Nordic Semiconductor
    //     0x503a,
    //     strings,
    // )
    // .finalize(components::keyboard_hid_component_static!(UsbHw));

    // keyboard_hid.enable();
    // keyboard_hid.attach();

    //--------------------------------------------------------------------------
    // PLATFORM SETUP, SCHEDULER, AND START KERNEL LOOP
    //--------------------------------------------------------------------------

    let scheduler = components::sched::round_robin::RoundRobinComponent::new(&*addr_of!(PROCESSES))
        .finalize(components::round_robin_component_static!(NUM_PROCS));

    let platform = Platform {
        // button: button,
        // ble_radio,
        pconsole,
        console,
        led,
        // gpio,
        rng,
        // adc,
        // temp,
        alarm,
        // analog_comparator,
        ipc: kernel::ipc::IPC::new(
            board_kernel,
            kernel::ipc::DRIVER_NUM,
            &memory_allocation_capability,
        ),
        // i2c_master_slave,
        // spi_controller,
        // kv_driver,
        nvmc: nvmc,
        usb,
        scheduler,
        systick: cortexm33::systick::SysTick::new_with_calibration(64000000),
        capacitive_touch: cap_touch_button,
    };

    // let _ = platform.pconsole.start();
    // base_peripherals.adc.calibrate();

    debug!("Initialization complete. Entering main loop\r");
    // debug!("{}", &*addr_of!(nrf5340::ficr::FICR_INSTANCE));

    (board_kernel, platform, chip, nrf5340_peripherals, mux_alarm)
}
