//! Component for CTAP over USB.

use capsules_extra::usb::usb_ctap::CtapUsbSyscallDriver;
use capsules_extra::usb::usbc_ctap_hid::ClientCtapHID;
use core::mem::MaybeUninit;
use kernel::capabilities;
use kernel::component::Component;
use kernel::create_capability;
use kernel::hil;

// Setup static space for the objects.
#[macro_export]
macro_rules! usb_ctap_component_helper {
    ($C:ty $(,)?) => {{
        use capsules_extra::usb::usb_ctap::CtapUsbSyscallDriver;
        use capsules_extra::usb::usbc_ctap_hid::ClientCtapHID;
        use core::mem::MaybeUninit;

        static mut hid: MaybeUninit<ClientCtapHID<'static, 'static, $C>> = MaybeUninit::uninit();
        static mut driver: MaybeUninit<CtapUsbSyscallDriver<'static, 'static, $C>> =
            MaybeUninit::uninit();

        (&mut hid, &mut driver)
    };};
}

pub struct UsbCtapComponent<C: 'static + hil::usb::UsbController<'static>> {
    board_kernel: &'static kernel::Kernel,
    driver_num: usize,
    controller: &'static C,
    max_ctrl_packet_size: u8,
    vendor_id: u16,
    product_id: u16,
    strings: &'static [&'static str],
}

impl<C: 'static + hil::usb::UsbController<'static>> UsbCtapComponent<C> {
    pub fn new(
        board_kernel: &'static kernel::Kernel,
        driver_num: usize,
        controller: &'static C,
        max_ctrl_packet_size: u8,
        vendor_id: u16,
        product_id: u16,
        strings: &'static [&'static str],
    ) -> Self {
        Self {
            board_kernel,
            driver_num,
            controller,
            max_ctrl_packet_size,
            vendor_id,
            product_id,
            strings,
        }
    }
}

impl<C: 'static + hil::usb::UsbController<'static>> Component for UsbCtapComponent<C> {
    type StaticInput = (
        &'static mut MaybeUninit<ClientCtapHID<'static, 'static, C>>,
        &'static mut MaybeUninit<CtapUsbSyscallDriver<'static, 'static, C>>,
    );
    type Output = &'static CtapUsbSyscallDriver<'static, 'static, C>;

    // unsafe fn finalize(self, s: Self::StaticInput) -> Self::Output {
    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);

        let usb_ctap = s.0.write(ClientCtapHID::new(
            self.controller,
            self.max_ctrl_packet_size,
            self.vendor_id,
            self.product_id,
            self.strings,
        ));
        self.controller.set_client(usb_ctap);

        // Configure the USB userspace driver
        let usb_driver = s.1.write(CtapUsbSyscallDriver::new(
            usb_ctap,
            self.board_kernel.create_grant(self.driver_num, &grant_cap),
        ));
        usb_ctap.set_client(usb_driver);

        usb_driver
    }
}
