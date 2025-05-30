use core::cell::Cell;
use kernel::debug;
use kernel::hil::gpio::{self, Configure, Input, InterruptEdge, InterruptPin, Output, Pin};
use kernel::hil::time::{Alarm, AlarmClient, Ticks};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;

pub struct CapacitiveTouchSensor<'a, A: Alarm<'a>> {
    // ... existing fields ...
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
}

#[derive(Copy, Clone, PartialEq)]
enum SensorState {
    Idle,
    Charging,
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
        }
    }

    pub fn set_pin_id(&self, id: u32) {
        self.pin_id.set(id);
    }

    pub fn get_pin_id(&self) -> u32 {
        self.pin_id.get()
    }

    pub fn start_measurement(&self) {
        // Configure pin as output and set high to charge the capacitor
        self.pin.make_output();
        self.pin.set();
        self.state.set(SensorState::Charging);

        // Set alarm for end of charging phase
        let now = self.alarm.now();
        // Increase charging time to ensure capacitor is fully charged
        let charge_time = A::Ticks::from(50); // Longer charging time
        self.alarm.set_alarm(now, charge_time);
    }

    fn start_discharge(&self) {
        // Match user space: input mode, pull-none (default)
        self.pin.make_input();
        self.pin.set_floating_state(gpio::FloatingState::PullNone);

        self.state.set(SensorState::Discharging);

        // Polling-based discharge measurement (user space style)
        let mut count: u32 = 0;
        // Busy-wait loop to poll the pin, incrementing count until LOW
        while self.pin.read() {
            // Insert a small busy-wait to stretch timing, as in user space
            for _ in 0..10 {}
            count += 1;
        }
        // Determine new touch state
        let was_touched = self.last_is_touched.get();
        let is_touched = count > self.threshold.get().into_u32();
        self.is_touched.set(is_touched);
        debug!(
            "[CapTouch-KERNEL-POLL] PIN:{} COUNT:{} THRESH:{}",
            self.pin_id.get(),
            count,
            self.threshold.get().into_u32()
        );

        // Only notify clients if the state has changed
        if is_touched != was_touched {
            self.last_is_touched.set(is_touched);
            if let Some(client) = self.client.get() {
                client.fired();
            }
            if let Some(client) = self.client_with_value.get() {
                client.fired(self.pin_id.get());
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

    /// Minimal stub to allow alarm() to call check_discharge without error.
    fn check_discharge(&self) {
        // For backward compatibility, just call start_discharge() (which does the polling and updates state)
        self.start_discharge();
    }

    pub fn enable(&self) {
        self.enabled.set(true);
        // Start scanning if currently idle
        if self.state.get() == SensorState::Idle {
            self.start_measurement();
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
            SensorState::Charging => {
                // Charging phase complete, start discharge
                self.start_discharge();
            }
            SensorState::Discharging => {
                // Check the pin state and process discharge
                self.check_discharge();
            }
            SensorState::Idle => {
                // Only start a new scan if explicitly enabled
                if self.enabled.get() {
                    // Time for next scan
                    self.start_measurement();
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
    fn set(&self) {
        // Not applicable for capacitive sensor
    }

    fn clear(&self) {
        // Not applicable for capacitive sensor
    }

    fn toggle(&self) -> bool {
        // Not applicable for capacitive sensor
        self.read()
    }
}

impl<'a, A: Alarm<'a>> Configure for CapacitiveTouchSensor<'a, A> {
    fn configuration(&self) -> gpio::Configuration {
        gpio::Configuration::Input
    }

    fn make_output(&self) -> gpio::Configuration {
        // For capacitive sensor, this doesn't change the actual configuration
        gpio::Configuration::Input
    }

    fn disable_output(&self) -> gpio::Configuration {
        // For capacitive sensor, this doesn't change the actual configuration
        gpio::Configuration::Input
    }

    fn make_input(&self) -> gpio::Configuration {
        // For capacitive sensor, this doesn't change the actual configuration
        gpio::Configuration::Input
    }

    fn disable_input(&self) -> gpio::Configuration {
        // For capacitive sensor, this doesn't change the actual configuration
        gpio::Configuration::Input
    }

    fn deactivate_to_low_power(&self) {
        // Put the pin in a low power state
        // For capacitive sensor, we can disable scanning
        self.disable();
    }

    fn set_floating_state(&self, _state: gpio::FloatingState) {
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
        // Reset the measurement completion flag
        self.measurement_completed.set(false);
        // Clear any pending disable requests
        self.disable_pending.set(false);
        // Enable scanning when interrupts are enabled
        self.enable();
    }

    fn disable_interrupts(&self) {
        if !self.measurement_completed.get() && self.state.get() != SensorState::Idle {
            // If a measurement is in progress and we haven't completed at least one measurement,
            // mark as pending but don't disable yet
            self.disable_pending.set(true);
        } else {
            // Either we've completed at least one measurement or we're idle
            self.disable();
        }
    }

    fn is_pending(&self) -> bool {
        // Not applicable for capacitive sensor
        false
    }
}

// Implementation of InterruptWithValue trait
impl<'a, A: Alarm<'a>> gpio::InterruptWithValue<'a> for CapacitiveTouchSensor<'a, A> {
    fn set_client(&self, client: &'a dyn gpio::ClientWithValue) {
        self.client_with_value.set(client);
    }

    fn enable_interrupts(&self, _mode: InterruptEdge) -> Result<(), ErrorCode> {
        // Reset the measurement completion flag
        self.measurement_completed.set(false);
        // Clear any pending disable requests
        self.disable_pending.set(false);
        // Enable scanning when interrupts are enabled
        self.enable();
        Ok(())
    }

    fn disable_interrupts(&self) {
        if !self.measurement_completed.get() && self.state.get() != SensorState::Idle {
            // If a measurement is in progress and we haven't completed at least one measurement,
            // mark as pending but don't disable yet
            self.disable_pending.set(true);
        } else {
            // Either we've completed at least one measurement or we're idle
            self.disable();
        }
    }

    fn is_pending(&self) -> bool {
        // Not applicable for capacitive sensor
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
    fn fired(&self, value: u32) {
        // This is called when the capacitive sensor is used as a client for another interrupt
    }
}
