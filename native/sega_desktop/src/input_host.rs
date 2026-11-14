use sega_input_runtime::{
    ActionBinding, ActionId, ActionState, AxisDirection, DeviceClass, DeviceDescriptor,
    DeviceReading, InputBackend, InputBuffers, InputProfile, PhysicalControl,
    sdl3_backend::{SDL3_RAW_AXIS_OFFSET, SDL3_RAW_BUTTON_OFFSET, Sdl3Backend},
};
use sega_sms_core::ControllerButtons;

const ACTION_UP: u8 = 0;
const ACTION_DOWN: u8 = 1;
const ACTION_LEFT: u8 = 2;
const ACTION_RIGHT: u8 = 3;
const ACTION_BUTTON_1: u8 = 4;
const ACTION_BUTTON_2: u8 = 5;
const ACTION_PAUSE_NMI: u8 = 6;
const DEFAULT_STICK_THRESHOLD: u16 = 10_000;

fn action(value: u8) -> ActionId {
    ActionId::new(value).expect("SMS input action identifier is in range")
}

fn binding(action_id: u8, control: PhysicalControl) -> ActionBinding {
    ActionBinding {
        action: action(action_id),
        control,
    }
}

/// Default mappings for SDL's standardized Gamepad API and generic Joystick API.
/// The profiles are built once and evaluated at the host polling boundary.
#[derive(Debug, Clone)]
struct SmsInputProfiles {
    gamepad: InputProfile,
    joystick: InputProfile,
}

impl SmsInputProfiles {
    fn defaults() -> Result<Self, String> {
        let gamepad = InputProfile::new(
            vec![
                binding(ACTION_UP, PhysicalControl::Button(11)),
                binding(ACTION_DOWN, PhysicalControl::Button(12)),
                binding(ACTION_LEFT, PhysicalControl::Button(13)),
                binding(ACTION_RIGHT, PhysicalControl::Button(14)),
                binding(ACTION_UP, PhysicalControl::Axis { index: 1, direction: AxisDirection::Negative, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_DOWN, PhysicalControl::Axis { index: 1, direction: AxisDirection::Positive, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_LEFT, PhysicalControl::Axis { index: 0, direction: AxisDirection::Negative, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_RIGHT, PhysicalControl::Axis { index: 0, direction: AxisDirection::Positive, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_BUTTON_1, PhysicalControl::Button(0)),
                binding(ACTION_BUTTON_2, PhysicalControl::Button(1)),
                binding(ACTION_PAUSE_NMI, PhysicalControl::Button(6)),
            ],
            0,
        )
        .map_err(|error| error.to_string())?;

        let joystick = InputProfile::new(
            vec![
                binding(ACTION_UP, PhysicalControl::Hat { index: 0, direction_mask: 0b0001 }),
                binding(ACTION_RIGHT, PhysicalControl::Hat { index: 0, direction_mask: 0b0010 }),
                binding(ACTION_DOWN, PhysicalControl::Hat { index: 0, direction_mask: 0b0100 }),
                binding(ACTION_LEFT, PhysicalControl::Hat { index: 0, direction_mask: 0b1000 }),
                binding(ACTION_UP, PhysicalControl::Axis { index: SDL3_RAW_AXIS_OFFSET as u8, direction: AxisDirection::Negative, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_DOWN, PhysicalControl::Axis { index: (SDL3_RAW_AXIS_OFFSET + 1) as u8, direction: AxisDirection::Positive, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_LEFT, PhysicalControl::Axis { index: SDL3_RAW_AXIS_OFFSET as u8, direction: AxisDirection::Negative, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_RIGHT, PhysicalControl::Axis { index: SDL3_RAW_AXIS_OFFSET as u8, direction: AxisDirection::Positive, threshold: DEFAULT_STICK_THRESHOLD }),
                binding(ACTION_BUTTON_1, PhysicalControl::Button(SDL3_RAW_BUTTON_OFFSET as u8)),
                binding(ACTION_BUTTON_2, PhysicalControl::Button((SDL3_RAW_BUTTON_OFFSET + 1) as u8)),
                binding(ACTION_PAUSE_NMI, PhysicalControl::Button((SDL3_RAW_BUTTON_OFFSET + 2) as u8)),
            ],
            0,
        )
        .map_err(|error| error.to_string())?;

        Ok(Self { gamepad, joystick })
    }

    fn evaluate(&self, class: DeviceClass, reading: &DeviceReading) -> (ControllerButtons, bool) {
        let profile = if class == DeviceClass::Gamepad {
            &self.gamepad
        } else {
            &self.joystick
        };
        let mut actions = ActionState::default();
        profile.evaluate(&reading.state, &mut actions);
        (
            ControllerButtons {
                up: actions.is_active(action(ACTION_UP)),
                down: actions.is_active(action(ACTION_DOWN)),
                left: actions.is_active(action(ACTION_LEFT)),
                right: actions.is_active(action(ACTION_RIGHT)),
                button_1: actions.is_active(action(ACTION_BUTTON_1)),
                button_2: actions.is_active(action(ACTION_BUTTON_2)),
            },
            actions.is_active(action(ACTION_PAUSE_NMI)),
        )
    }
}

pub struct Sdl3InputHost {
    backend: Sdl3Backend,
    buffers: InputBuffers,
    devices: Vec<DeviceDescriptor>,
    profiles: SmsInputProfiles,
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
            profiles: SmsInputProfiles::defaults()?,
            status: String::new(),
        };
        host.refresh_status();
        Ok(host)
    }

    /// Replaces the active standardized-gamepad and generic-joystick profiles.
    /// The UI can later load persisted profiles and pass them through this API.
    pub fn set_profiles(
        &mut self,
        gamepad: InputProfile,
        joystick: InputProfile,
    ) {
        self.profiles = SmsInputProfiles { gamepad, joystick };
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
                let (buttons, pause) = self.profiles.evaluate(descriptor.class, reading);
                players[current_slot] = buttons;
                pause_pressed |= pause;
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

#[cfg(test)]
mod tests {
    use super::{DEFAULT_STICK_THRESHOLD, SmsInputProfiles};
    use sega_input_runtime::{
        ActionBinding, ActionId, AxisDirection, AxisSemantic, DeviceClass, DeviceId,
        DeviceReading, DeviceState, InputProfile, PhysicalControl,
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
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(0, true).unwrap();
        state.buttons.set(1, true).unwrap();
        state.buttons.set(11, true).unwrap();
        state
            .set_axis(0, DEFAULT_STICK_THRESHOLD as i16, AxisSemantic::LeftStickX)
            .unwrap();

        let (buttons, pause) = profiles.evaluate(DeviceClass::Gamepad, &reading(state));
        assert!(buttons.up);
        assert!(buttons.right);
        assert!(buttons.button_1);
        assert!(buttons.button_2);
        assert!(!buttons.down);
        assert!(!buttons.left);
        assert!(!pause);
    }

    #[test]
    fn generic_joystick_maps_raw_buttons_axes_and_hat() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(32, true).unwrap();
        state.buttons.set(33, true).unwrap();
        state
            .set_axis(16, -(DEFAULT_STICK_THRESHOLD as i16), AxisSemantic::Unknown)
            .unwrap();
        state.set_hat(0, 0b0100).unwrap();

        let (buttons, pause) = profiles.evaluate(DeviceClass::Joystick, &reading(state));
        assert!(buttons.left);
        assert!(buttons.down);
        assert!(buttons.button_1);
        assert!(buttons.button_2);
        assert!(!buttons.up);
        assert!(!buttons.right);
        assert!(!pause);
    }

    #[test]
    fn gamepad_start_maps_to_sms_pause_nmi() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(6, true).unwrap();

        let (_, pause) = profiles.evaluate(DeviceClass::Gamepad, &reading(state));
        assert!(pause);
    }

    #[test]
    fn generic_joystick_third_button_maps_to_sms_pause_nmi() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(34, true).unwrap();

        let (_, pause) = profiles.evaluate(DeviceClass::Joystick, &reading(state));
        assert!(pause);
    }

    #[test]
    fn analog_center_noise_does_not_activate_directions() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.set_axis(0, 8_000, AxisSemantic::LeftStickX).unwrap();
        state.set_axis(1, -8_000, AxisSemantic::LeftStickY).unwrap();

        let (buttons, _) = profiles.evaluate(DeviceClass::Gamepad, &reading(state));
        assert_eq!(buttons, Default::default());
    }

    #[test]
    fn custom_profile_can_move_sms_button_to_an_extra_gamepad_button() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let custom = InputProfile::new(
            vec![ActionBinding {
                action: ActionId::new(4).unwrap(),
                control: PhysicalControl::Button(8),
            }],
            0,
        )
        .unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(8, true).unwrap();
        let mut actions = sega_input_runtime::ActionState::default();
        custom.evaluate(&state, &mut actions);
        assert!(actions.is_active(ActionId::new(4).unwrap()));
        assert_eq!(profiles.gamepad.deadzone(), 0);
    }
}
