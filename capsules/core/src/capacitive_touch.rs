use core::cell::Cell;
use kernel::debug;
use kernel::hil::gpio::{self, Configure, Input, InterruptEdge, InterruptPin, Output, Pin};
use kernel::hil::time::{Alarm, AlarmClient, Ticks};
use kernel::utilities::cells::OptionalCell;
use kernel::ErrorCode;

pub struct CapacitiveTouchSensor<'a, A: Alarm<'a>> {
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
        }
    }

    pub fn set_pin_id(&self, id: u32) {
        self.pin_id.set(id);
    }

    pub fn get_pin_id(&self) -> u32 {
        self.pin_id.get()
    }

    pub fn start_measurement(&self) {
        debug!(
            "[CapTouch] Starting measurement for pin {}",
            self.pin_id.get()
        );
        // Configure pin as output and set high to charge the capacitor
        self.pin.make_output();
        self.pin.set();
        self.state.set(SensorState::Charging);

        // Set alarm for end of charging phase
        let now = self.alarm.now();
        let charge_time = A::Ticks::from(10); // Short charging time, adjust as needed
        debug!("[CapTouch] Charging for {} ticks", charge_time.into_u32());
        self.alarm.set_alarm(now, charge_time);
    }

    fn start_discharge(&self) {
        debug!(
            "[CapTouch] Starting discharge phase for pin {}",
            self.pin_id.get()
        );
        // Switch to input mode to let the capacitor discharge
        self.pin.make_input();
        self.state.set(SensorState::Discharging);

        // Start measuring discharge time
        self.next_scan.set(self.alarm.now());
        debug!(
            "[CapTouch] Discharge measurement started at tick {}",
            self.next_scan.get().into_u32()
        );
    }

    fn check_discharge(&self) -> bool {
        // Check if pin has discharged (gone low)
        let is_discharged = !self.pin.read();
        debug!(
            "[CapTouch] Pin {} discharged: {}",
            self.pin_id.get(),
            is_discharged
        );

        if is_discharged {
            // Calculate discharge time
            let discharge_time = self.alarm.now().wrapping_sub(self.next_scan.get());
            debug!(
                "[CapTouch] Discharge time: {} ticks",
                discharge_time.into_u32()
            );

            // Determine if touched based on threshold
            // Compare with threshold to determine if touched
            let threshold = self.threshold.get();
            let was_touched = self.is_touched.get();
            let is_touched = discharge_time > threshold;
            debug!(
                "[CapTouch] Pin {} - Discharge time: {} ticks, Threshold: {} ticks, Touched: {}",
                self.pin_id.get(),
                discharge_time.into_u32(),
                threshold.into_u32(),
                is_touched
            );
            self.is_touched.set(is_touched);
            if let Some(client) = self.client.get() {
                client.fired();
            }
            if let Some(client) = self.client_with_value.get() {
                client.fired(self.pin_id.get());
            }

            // Return to idle state
            self.state.set(SensorState::Idle);

            // Schedule next scan
            let now = self.alarm.now();
            self.next_scan
                .set(now.wrapping_add(self.scan_interval.get()));

            true
        } else {
            // Not discharged yet, check timeout
            let now = self.alarm.now();
            if now.wrapping_sub(self.next_scan.get()) > A::Ticks::from(1000) {
                // Timeout value, adjust as needed
                // Timeout - assume not touched
                if self.is_touched.get() {
                    self.is_touched.set(false);
                    if let Some(client) = self.client.get() {
                        client.fired();
                    }
                    if let Some(client) = self.client_with_value.get() {
                        client.fired(self.pin_id.get());
                    }
                }

                // Return to idle state
                self.state.set(SensorState::Idle);

                // Schedule next scan
                self.next_scan
                    .set(now.wrapping_add(self.scan_interval.get()));

                true
            } else {
                false
            }
        }
    }

    pub fn enable(&self) {
        debug!(
            "[CapTouch] Enabling capacitive touch sensor on pin {}",
            self.pin_id.get()
        );
        self.enabled.set(true);
        // Start scanning if currently idle
        if self.state.get() == SensorState::Idle {
            self.start_measurement();
        }
    }

    pub fn disable(&self) {
        debug!(
            "[CapTouch] Disabling capacitive touch sensor on pin {}",
            self.pin_id.get()
        );
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
                debug!(
                    "[CapTouch] Alarm fired: Charging phase complete for pin {}",
                    self.pin_id.get()
                );
                // Charging phase complete, start discharge
                self.start_discharge();
            }
            SensorState::Discharging => {
                debug!(
                    "[CapTouch] Alarm fired: Checking discharge for pin {}",
                    self.pin_id.get()
                );
                // Check if discharged
                if !self.check_discharge() {
                    // Not discharged yet, check again soon
                    let now = self.alarm.now();
                    debug!(
                        "[CapTouch] Pin {} not discharged yet, checking again in 1 tick",
                        self.pin_id.get()
                    );
                    self.alarm.set_alarm(now, A::Ticks::from(1));
                } else {
                    // Discharge measurement complete, schedule next scan
                    self.state.set(SensorState::Idle);
                    let now = self.alarm.now();
                    let next = now.wrapping_add(self.scan_interval.get());
                    debug!(
                        "[CapTouch] Pin {} discharge complete, next scan in {} ticks",
                        self.pin_id.get(),
                        self.scan_interval.get().into_u32()
                    );
                    self.alarm.set_alarm(next, A::Ticks::from(0));
                }
            }
            SensorState::Idle => {
                // Only start a new scan if explicitly enabled
                if self.enabled.get() {
                    debug!(
                        "[CapTouch] Alarm fired: Starting next scan for pin {}",
                        self.pin_id.get()
                    );
                    // Time for next scan
                    self.start_measurement();
                } else {
                    debug!(
                        "[CapTouch] Alarm fired but scanning is disabled for pin {}",
                        self.pin_id.get()
                    );
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
        debug!(
            "[CapTouch] Enable interrupts called for pin {}, starting scanning",
            self.pin_id.get()
        );
        // Enable scanning when interrupts are enabled
        self.enable();
    }

    fn disable_interrupts(&self) {
        debug!(
            "[CapTouch] Disable interrupts called for pin {}, stopping scanning",
            self.pin_id.get()
        );
        // Disable scanning when interrupts are disabled
        self.disable();
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
        debug!(
            "[CapTouch] Enable interrupts with value called for pin {}, starting scanning",
            self.pin_id.get()
        );
        // Enable scanning when interrupts are enabled
        self.enable();
        Ok(())
    }

    fn disable_interrupts(&self) {
        debug!(
            "[CapTouch] Disable interrupts with value called for pin {}, stopping scanning",
            self.pin_id.get()
        );
        // Disable scanning when interrupts are disabled
        self.disable();
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
        // This method is called when this sensor is used as a client for another interrupt source
        // We can use this to handle external events that might affect the sensor
        // For now, we just pass it to our own client if we have one
        if let Some(client) = self.client_with_value.get() {
            client.fired(value);
        }
    }
}
