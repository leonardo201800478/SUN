use sega_input_runtime::{
    DeviceClass, DeviceDescriptor, DeviceReading, InputBackend, InputBuffers,
    sdl3_backend::{SDL3_RAW_AXIS_OFFSET, SDL3_RAW_BUTTON_OFFSET, Sdl3Backend},
};
use sega_sms_core::ControllerButtons;

const GAMEPAD_BUTTON_SOUTH: usize = 0;
const GAMEPAD_BUTTON_EAST: usize = 1;
const GAMEPAD_BUTTON_START: usize = 6;
const GAMEPAD_BUTTON_DPAD_UP: usize = 11;
const GAMEPAD_BUTTON_DPAD_DOWN: usize = 12;
const GAMEPAD_BUTTON_DPAD_LEFT: usize = 13;
const GAMEPAD_BUTTON_DPAD_RIGHT: usize = 14;
const GAMEPAD_AXIS_LEFT_X: usize = 0;
const GAMEPAD_AXIS_LEFT_Y: usize = 1;

// SDL documents a center tolerance of roughly 8,000 units for thumbsticks.
// 16,000 avoids treating ordinary center noise as an intentional direction.
const DEFAULT_STICK_THRESHOLD: i32 = 16_000;

// The current backend keeps standardized gamepad buttons in 0..21 and raw
// joystick buttons/axes in separate ranges to avoid collisions.
pub struct Sdl3InputHost {
    backend: Sdl3Backend,
    buffers: InputBuffers,
    devices: Vec<DeviceDescriptor>,
    status: String,
}

impl Sdl3InputHost {
    pub fn new() -> Result<Self, String> {
        let mut backend = Sdl3Backend::new().map_err(|error| error.to_string())?;
        let mut devices = Vec::new();
        backend
            .enumerate(&mut devices)
            .map_err(|error| error.to_string())?;

        let mut host = Self {
            backend,
            buffers: InputBuffers::default(),
            devices,
            status: String::new(),
        };
        host.refresh_status();
        Ok(host)
    }

    /// Polls SDL3 at the host boundary and assigns the first two supported
    /// physical devices, in session order, to SMS Player 1 and Player 2.
    /// This is deliberately independent of MachineScheduler progression.
    pub fn poll(&mut self) -> Result<([ControllerButtons; 2], bool, Option<String>), String> {
        self.buffers.clear();
        self.backend
            .poll_into(&mut self.buffers)
            .map_err(|error| error.to_string())?;

        let status_update = if !self.buffers.events.is_empty() {
            self.backend
                .enumerate(&mut self.devices)
                .map_err(|error| error.to_string())?;
            self.refresh_status();
            Some(self.status.clone())
        } else {
            None
        };

        let mut players = [ControllerButtons::default(); 2];
        let mut slot = 0;
        let mut pause_pressed = false;

        for descriptor in &self.devices {
            if !is_supported_controller(descriptor.class) {
                continue;
            }
            let current_slot = slot;
            slot += 1;
            if let Some(reading) = self
                .buffers
                .readings
                .iter()
                .find(|reading| reading.device_id == descriptor.id)
            {
                players[current_slot] = map_sms_controller(descriptor.class, reading);
                pause_pressed |= map_sms_pause(descriptor.class, reading);
            }
            if slot == players.len() {
                break;
            }
        }

        Ok((players, pause_pressed, status_update))
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    fn refresh_status(&mut self) {
        let mut count = 0;
        let mut names = String::new();
        for device in &self.devices {
            if !is_supported_controller(device.class) {
                continue;
            }
            count += 1;
            if count <= 2 {
                if !names.is_empty() {
                    names.push_str(", ");
                }
                names.push_str(&device.name);
            }
        }
        self.status = if count == 0 {
            "SDL3 ativo — nenhum gamepad/joystick compatível conectado".to_owned()
        } else {
            format!("SDL3 ativo — {count} controle(s): {names}")
        };
    }
}

fn is_supported_controller(class: DeviceClass) -> bool {
    matches!(
        class,
        DeviceClass::Gamepad | DeviceClass::Joystick | DeviceClass::ArcadeStick
    )
}

fn map_sms_pause(class: DeviceClass, reading: &DeviceReading) -> bool {
    if class == DeviceClass::Gamepad {
        reading.state.buttons.is_pressed(GAMEPAD_BUTTON_START)
    } else {
        reading
            .state
            .buttons
            .is_pressed(SDL3_RAW_BUTTON_OFFSET + 2)
    }
}

fn map_sms_controller(class: DeviceClass, reading: &DeviceReading) -> ControllerButtons {
    let state = &reading.state;

    if class == DeviceClass::Gamepad {
        let left_x = i32::from(state.axis(GAMEPAD_AXIS_LEFT_X));
        let left_y = i32::from(state.axis(GAMEPAD_AXIS_LEFT_Y));
        return ControllerButtons {
            up: state.buttons.is_pressed(GAMEPAD_BUTTON_DPAD_UP)
                || left_y <= -DEFAULT_STICK_THRESHOLD,
            down: state.buttons.is_pressed(GAMEPAD_BUTTON_DPAD_DOWN)
                || left_y >= DEFAULT_STICK_THRESHOLD,
            left: state.buttons.is_pressed(GAMEPAD_BUTTON_DPAD_LEFT)
                || left_x <= -DEFAULT_STICK_THRESHOLD,
            right: state.buttons.is_pressed(GAMEPAD_BUTTON_DPAD_RIGHT)
                || left_x >= DEFAULT_STICK_THRESHOLD,
            button_1: state.buttons.is_pressed(GAMEPAD_BUTTON_SOUTH),
            button_2: state.buttons.is_pressed(GAMEPAD_BUTTON_EAST),
        };
    }

    let raw_x = i32::from(state.axis(SDL3_RAW_AXIS_OFFSET));
    let raw_y = i32::from(state.axis(SDL3_RAW_AXIS_OFFSET + 1));
    let hat = state.hat(0);

    ControllerButtons {
        up: hat & 0b0001 != 0 || raw_y <= -DEFAULT_STICK_THRESHOLD,
        right: hat & 0b0010 != 0 || raw_x >= DEFAULT_STICK_THRESHOLD,
        down: hat & 0b0100 != 0 || raw_y >= DEFAULT_STICK_THRESHOLD,
        left: hat & 0b1000 != 0 || raw_x <= -DEFAULT_STICK_THRESHOLD,
        button_1: state.buttons.is_pressed(SDL3_RAW_BUTTON_OFFSET),
        button_2: state.buttons.is_pressed(SDL3_RAW_BUTTON_OFFSET + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_STICK_THRESHOLD, SDL3_RAW_AXIS_OFFSET, SDL3_RAW_BUTTON_OFFSET,
        map_sms_controller, map_sms_pause,
    };
    use sega_input_runtime::{
        AxisSemantic, ButtonSet, DeviceClass, DeviceId, DeviceReading, DeviceState,
    };

    fn reading(state: DeviceState) -> DeviceReading {
        DeviceReading {
            device_id: DeviceId(1),
            timestamp_ns: 10,
            sequence: 1,
            state,
        }
    }

    #[test]
    fn standardized_gamepad_maps_south_east_dpad_and_left_stick() {
        let mut state = DeviceState::default();
        state.buttons.set(0, true).unwrap();
        state.buttons.set(1, true).unwrap();
        state.buttons.set(11, true).unwrap();
        state
            .set_axis(0, DEFAULT_STICK_THRESHOLD as i16, AxisSemantic::LeftStickX)
            .unwrap();

        let buttons = map_sms_controller(DeviceClass::Gamepad, &reading(state));
        assert!(buttons.up);
        assert!(buttons.right);
        assert!(buttons.button_1);
        assert!(buttons.button_2);
        assert!(!buttons.down);
        assert!(!buttons.left);
    }

    #[test]
    fn generic_joystick_maps_raw_buttons_axes_and_hat() {
        let mut state = DeviceState::default();
        let mut raw_buttons = ButtonSet::default();
        raw_buttons.set(SDL3_RAW_BUTTON_OFFSET, true).unwrap();
        raw_buttons.set(SDL3_RAW_BUTTON_OFFSET + 1, true).unwrap();
        state.buttons = raw_buttons;
        state
            .set_axis(
                SDL3_RAW_AXIS_OFFSET,
                -(DEFAULT_STICK_THRESHOLD as i16),
                AxisSemantic::Unknown,
            )
            .unwrap();
        state.set_hat(0, 0b0100).unwrap();

        let buttons = map_sms_controller(DeviceClass::Joystick, &reading(state));
        assert!(buttons.left);
        assert!(buttons.down);
        assert!(buttons.button_1);
        assert!(buttons.button_2);
        assert!(!buttons.up);
        assert!(!buttons.right);
    }

    #[test]
    fn gamepad_start_maps_to_sms_pause_nmi() {
        let mut state = DeviceState::default();
        state.buttons.set(6, true).unwrap();

        assert!(map_sms_pause(DeviceClass::Gamepad, &reading(state)));
    }

    #[test]
    fn generic_joystick_third_button_maps_to_sms_pause_nmi() {
        let mut state = DeviceState::default();
        let mut raw_buttons = ButtonSet::default();
        raw_buttons.set(SDL3_RAW_BUTTON_OFFSET + 2, true).unwrap();
        state.buttons = raw_buttons;

        assert!(map_sms_pause(DeviceClass::Joystick, &reading(state)));
    }

    #[test]
    fn analog_center_noise_does_not_activate_directions() {
        let mut state = DeviceState::default();
        state.set_axis(0, 8_000, AxisSemantic::LeftStickX).unwrap();
        state.set_axis(1, -8_000, AxisSemantic::LeftStickY).unwrap();

        let buttons = map_sms_controller(DeviceClass::Gamepad, &reading(state));
        assert_eq!(buttons, Default::default());
    }
}
