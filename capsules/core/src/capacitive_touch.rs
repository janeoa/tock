use core::cell::Cell;
use kernel::deferred_call::DeferredCallClient;
use kernel::hil::gpio::{self, Configure, Input, InterruptEdge, Output, Pin};
use kernel::hil::time::{Alarm, AlarmClient, Ticks};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;

pub struct CapacitiveTouchSensor<'a, A: Alarm<'a>> {
    /// Last reported touch state (for edge detection)
    last_is_touched: Cell<bool>,
    // The GPIO pin used for the capacitive sensor
    pin: &'a dyn Pin,

    // Timer for measuring discharge time
    alarm: &'a A,

    // Current state of the sensor
    state: Cell<SensorState>,

    // Whether the sensor is currently considered "touched"
    is_touched: Cell<bool>,

    // Threshold for determining touch (in timer ticks)
    threshold: Cell<A::Ticks>,

    // Client to notify of interrupt events
    client: OptionalCell<&'a dyn gpio::Client>,

    // Client with value to notify of interrupt events with a value
    client_with_value: OptionalCell<&'a dyn gpio::ClientWithValue>,

    // For periodic scanning
    next_scan: Cell<A::Ticks>,

    // Scan interval (in timer ticks)
    scan_interval: Cell<A::Ticks>,

    // Pin number or identifier for this sensor
    pin_id: Cell<u32>,

    // Whether scanning is enabled
    enabled: Cell<bool>,

    // Flag to track if a disable request is pending (waiting for measurement to complete)
    disable_pending: Cell<bool>,

    // Flag to track if at least one measurement has completed
    measurement_completed: Cell<bool>,

    // Deferred callback to prevent re-entering grant regions
    deferred_callback: kernel::deferred_call::DeferredCall,
}

#[derive(Copy, Clone, PartialEq)]
enum SensorState {
    Idle,
    Discharging,
}

impl<'a, A: Alarm<'a>> CapacitiveTouchSensor<'a, A> {
    pub fn new(
        pin: &'a dyn Pin,
        alarm: &'a A,
        threshold: A::Ticks,
        scan_interval: A::Ticks,
    ) -> Self {
        Self {
            pin,
            alarm,
            state: Cell::new(SensorState::Idle),
            is_touched: Cell::new(false),
            threshold: Cell::new(threshold),
            client: OptionalCell::empty(),
            client_with_value: OptionalCell::empty(),
            next_scan: Cell::new(A::Ticks::from(0)),
            scan_interval: Cell::new(scan_interval),
            pin_id: Cell::new(0),
            enabled: Cell::new(false),
            disable_pending: Cell::new(false),
            measurement_completed: Cell::new(false),
            last_is_touched: Cell::new(false),
            deferred_callback: kernel::deferred_call::DeferredCall::new(),
        }
    }

    pub fn set_pin_id(&self, id: u32) {
        self.pin_id.set(id);
    }

    pub fn get_pin_id(&self) -> u32 {
        self.pin_id.get()
    }

    pub fn start_measurement(&self) {
        self.pin.make_output();
        self.pin.set();

        self.pin.make_input();

        self.pin.make_output();
        self.pin.set();
    }

    fn start_discharge(&self) {
        self.pin.make_input();
        self.pin.set_floating_state(gpio::FloatingState::PullNone);
        self.state.set(SensorState::Discharging);

        let mut count: u32 = 0;
        while self.pin.read() {
            count += 1;
        }

        let was_touched = self.last_is_touched.get();
        let is_touched = count > self.threshold.get().into_u32();
        self.is_touched.set(is_touched);

        // Only notify clients if the state has changed, but use deferred callback
        if is_touched != was_touched {
            self.last_is_touched.set(is_touched);
            // Schedule the deferred callback instead of directly calling clients
            if !self.deferred_callback.is_pending() {
                self.deferred_callback.set();
            }
        }
        self.measurement_completed.set(true);
        self.state.set(SensorState::Idle);
        if self.disable_pending.get() {
            self.disable_pending.set(false);
            self.disable();
        } else {
            // Schedule next scan
            let now = self.alarm.now();
            let next = now.wrapping_add(self.scan_interval.get());
            self.alarm.set_alarm(now, self.scan_interval.get());
            self.next_scan.set(next);
        }
    }

    pub fn enable(&self) {
        self.enabled.set(true);
        // Start scanning if currently idle
        if self.state.get() == SensorState::Idle {
            self.start_measurement();
            self.start_discharge();
        }
    }

    pub fn disable(&self) {
        // Stop scanning
        self.enabled.set(false);
        self.state.set(SensorState::Idle);
        self.next_scan.set(A::Ticks::max_value());
    }
}

impl<'a, A: Alarm<'a>> AlarmClient for CapacitiveTouchSensor<'a, A> {
    fn alarm(&self) {
        match self.state.get() {
            SensorState::Discharging => {}
            SensorState::Idle => {
                // Only start a new scan if explicitly enabled
                if self.enabled.get() {
                    self.start_measurement();
                    self.start_discharge();
                }
            }
        }
    }
}

// Implementation of Pin trait (required for InterruptPin)
impl<'a, A: Alarm<'a>> Input for CapacitiveTouchSensor<'a, A> {
    fn read(&self) -> bool {
        self.is_touched.get()
    }
}

impl<'a, A: Alarm<'a>> Output for CapacitiveTouchSensor<'a, A> {
    fn set(&self) {}

    fn clear(&self) {}

    fn toggle(&self) -> bool {
        self.read()
    }
}

impl<'a, A: Alarm<'a>> Configure for CapacitiveTouchSensor<'a, A> {
    fn configuration(&self) -> gpio::Configuration {
        gpio::Configuration::Input
    }

    fn make_output(&self) -> gpio::Configuration {
        gpio::Configuration::Input
    }

    fn disable_output(&self) -> gpio::Configuration {
        gpio::Configuration::Input
    }

    fn make_input(&self) -> gpio::Configuration {
        gpio::Configuration::Input
    }

    fn disable_input(&self) -> gpio::Configuration {
        gpio::Configuration::Input
    }

    fn deactivate_to_low_power(&self) {
        self.disable();
    }

    fn set_floating_state(&self, state: gpio::FloatingState) {
        self.pin.set_floating_state(state);
        // Not applicable for capacitive sensor
    }

    fn floating_state(&self) -> gpio::FloatingState {
        gpio::FloatingState::PullNone
    }
}

// Implementation of Interrupt trait
impl<'a, A: Alarm<'a>> gpio::Interrupt<'a> for CapacitiveTouchSensor<'a, A> {
    fn set_client(&self, client: &'a dyn gpio::Client) {
        self.client.set(client);
    }

    fn enable_interrupts(&self, _mode: InterruptEdge) {
        self.measurement_completed.set(false);
        self.disable_pending.set(false);
        self.enable();
    }

    fn disable_interrupts(&self) {
        if !self.measurement_completed.get() && self.state.get() != SensorState::Idle {
            self.disable_pending.set(true);
        } else {
            self.disable();
        }
    }

    fn is_pending(&self) -> bool {
        false
    }
}

// Implementation of InterruptWithValue trait
impl<'a, A: Alarm<'a>> gpio::InterruptWithValue<'a> for CapacitiveTouchSensor<'a, A> {
    fn set_client(&self, client: &'a dyn gpio::ClientWithValue) {
        self.client_with_value.set(client);
    }

    fn enable_interrupts(&self, _mode: InterruptEdge) -> Result<(), ErrorCode> {
        self.measurement_completed.set(false);
        self.disable_pending.set(false);
        self.enable();
        Ok(())
    }

    fn disable_interrupts(&self) {
        self.disable();
    }

    fn is_pending(&self) -> bool {
        false
    }

    fn value(&self) -> u32 {
        self.pin_id.get()
    }

    fn set_value(&self, value: u32) {
        self.pin_id.set(value);
    }
}

// Implementation of ClientWithValue trait
impl<'a, A: Alarm<'a>> gpio::ClientWithValue for CapacitiveTouchSensor<'a, A> {
    fn fired(&self, _value: u32) {}
}

// Implementation for DeferredCallClient trait
impl<'a, A: Alarm<'a>> DeferredCallClient for CapacitiveTouchSensor<'a, A> {
    fn handle_deferred_call(&self) {
        // Notify clients here, outside of any critical sections
        if let Some(client) = self.client.get() {
            client.fired();
        }

        if let Some(client) = self.client_with_value.get() {
            client.fired(self.pin_id.get());
        }
    }

    fn register(&'static self) {
        self.deferred_callback.register(self);
    }
}
