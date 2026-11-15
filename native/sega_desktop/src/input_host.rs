use sega_input_runtime::{
    ActionBinding, ActionId, ActionState, AxisDirection, DeviceClass, DeviceDescriptor,
    DeviceReading, InputBackend, InputBuffers, InputProfile, PhysicalControl,
    sdl3_backend::{SDL3_RAW_BUTTON_OFFSET, Sdl3Backend},
};
use sega_sms_core::ControllerButtons;

/// Stable semantic identifiers shared by desktop configuration and SMS input profiles.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SmsInputAction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    Button1 = 4,
    Button2 = 5,
    PauseNmi = 6,
}

impl SmsInputAction {
    /// Numeric identifier consumed by the generic input-profile evaluator.
    pub const fn id(self) -> u8 {
        self as u8
    }
}

const DEFAULT_STICK_THRESHOLD: u16 = 10_000;

fn action(value: SmsInputAction) -> ActionId {
    ActionId::new(value.id()).expect("SMS input action identifier is in range")
}

fn binding(action_id: SmsInputAction, control: PhysicalControl) -> ActionBinding {
    ActionBinding {
        action: action(action_id),
        control,
    }
}

/// Default mappings for SDL's standardized Gamepad API and generic Joystick API.
/// The profiles are built once and evaluated at the host polling boundary.
#[derive(Debug, Clone)]
struct SmsInputProfiles {
    gamepad: [InputProfile; 2],
    joystick: [InputProfile; 2],
}

impl SmsInputProfiles {
    fn defaults() -> Result<Self, String> {
        let gamepad = InputProfile::new(
            vec![
                binding(SmsInputAction::Up, PhysicalControl::Button(11)),
                binding(SmsInputAction::Down, PhysicalControl::Button(12)),
                binding(SmsInputAction::Left, PhysicalControl::Button(13)),
                binding(SmsInputAction::Right, PhysicalControl::Button(14)),
                binding(
                    SmsInputAction::Up,
                    PhysicalControl::Axis {
                        index: 1,
                        direction: AxisDirection::Negative,
                        threshold: DEFAULT_STICK_THRESHOLD,
                    },
                ),
                binding(
                    SmsInputAction::Down,
                    PhysicalControl::Axis {
                        index: 1,
                        direction: AxisDirection::Positive,
                        threshold: DEFAULT_STICK_THRESHOLD,
                    },
                ),
                binding(
                    SmsInputAction::Left,
                    PhysicalControl::Axis {
                        index: 0,
                        direction: AxisDirection::Negative,
                        threshold: DEFAULT_STICK_THRESHOLD,
                    },
                ),
                binding(
                    SmsInputAction::Right,
                    PhysicalControl::Axis {
                        index: 0,
                        direction: AxisDirection::Positive,
                        threshold: DEFAULT_STICK_THRESHOLD,
                    },
                ),
                binding(SmsInputAction::Button1, PhysicalControl::Button(0)),
                binding(SmsInputAction::Button2, PhysicalControl::Button(1)),
                binding(SmsInputAction::PauseNmi, PhysicalControl::Button(6)),
            ],
            0,
        )
        .map_err(|error| error.to_string())?;

        let joystick = InputProfile::new(
            vec![
                binding(
                    SmsInputAction::Up,
                    PhysicalControl::Hat {
                        index: 0,
                        direction_mask: 0b0001,
                    },
                ),
                binding(
                    SmsInputAction::Right,
                    PhysicalControl::Hat {
                        index: 0,
                        direction_mask: 0b0010,
                    },
                ),
                binding(
                    SmsInputAction::Down,
                    PhysicalControl::Hat {
                        index: 0,
                        direction_mask: 0b0100,
                    },
                ),
                binding(
                    SmsInputAction::Left,
                    PhysicalControl::Hat {
                        index: 0,
                        direction_mask: 0b1000,
                    },
                ),
                // Generic joystick axis ordering is device-specific. Do not assume
                // axes 0/1 are a directional stick; calibrated profiles may bind them.
                binding(
                    SmsInputAction::Button1,
                    PhysicalControl::Button(SDL3_RAW_BUTTON_OFFSET as u8),
                ),
                binding(
                    SmsInputAction::Button2,
                    PhysicalControl::Button((SDL3_RAW_BUTTON_OFFSET + 1) as u8),
                ),
                binding(
                    SmsInputAction::PauseNmi,
                    PhysicalControl::Button((SDL3_RAW_BUTTON_OFFSET + 2) as u8),
                ),
            ],
            0,
        )
        .map_err(|error| error.to_string())?;

        Ok(Self {
            gamepad: [gamepad.clone(), gamepad],
            joystick: [joystick.clone(), joystick],
        })
    }

    fn set_profile(&mut self, player: usize, class: DeviceClass, profile: InputProfile) -> Result<(), String> {
        if player >= 2 {
            return Err("SMS player index must be 0 or 1".to_owned());
        }
        let profiles = if class == DeviceClass::Gamepad {
            &mut self.gamepad
        } else {
            &mut self.joystick
        };
        profiles[player] = profile;
        Ok(())
    }

    fn evaluate(&self, player: usize, class: DeviceClass, reading: &DeviceReading) -> (ControllerButtons, bool) {
        let profiles = if class == DeviceClass::Gamepad {
            &self.gamepad
        } else {
            &self.joystick
        };
        let profile = &profiles[player.min(1)];
        let mut actions = ActionState::default();
        profile.evaluate(&reading.state, &mut actions);
        (
            ControllerButtons {
                up: actions.is_active(action(SmsInputAction::Up)),
                down: actions.is_active(action(SmsInputAction::Down)),
                left: actions.is_active(action(SmsInputAction::Left)),
                right: actions.is_active(action(SmsInputAction::Right)),
                button_1: actions.is_active(action(SmsInputAction::Button1)),
                button_2: actions.is_active(action(SmsInputAction::Button2)),
            },
            actions.is_active(action(SmsInputAction::PauseNmi)),
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
                let (buttons, pause) = self.profiles.evaluate(current_slot, descriptor.class, reading);
                players[current_slot] = buttons;
                pause_pressed |= pause;
            }
            if slot == players.len() {
                break;
            }
        }

        Ok((players, pause_pressed, status_update))
    }

    /// Replaces one player's mapping for a controller class without restarting SDL3.
    pub fn set_player_profile(
        &mut self,
        player: usize,
        class: DeviceClass,
        profile: InputProfile,
    ) -> Result<(), String> {
        self.profiles.set_profile(player, class, profile)
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
    use super::{
        AxisDirection, DEFAULT_STICK_THRESHOLD, SmsInputAction, SmsInputProfiles, binding,
    };
    use sega_input_runtime::{
        AxisSemantic, DeviceClass, DeviceId, DeviceReading, DeviceState, InputProfile,
        PhysicalControl,
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

        let (buttons, pause) = profiles.evaluate(0, DeviceClass::Gamepad, &reading(state));
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
        state
            .set_axis(17, -(DEFAULT_STICK_THRESHOLD as i16), AxisSemantic::Unknown)
            .unwrap();
        // Hat DOWN and vertical-axis UP are simultaneous; profile bindings OR
        // controls targeting the same semantic action without suppressing either.
        state.set_hat(0, 0b0101).unwrap();

        let (buttons, pause) = profiles.evaluate(0, DeviceClass::Joystick, &reading(state));
        assert!(
            !buttons.left,
            "unconfigured generic axes must not be guessed"
        );
        assert!(buttons.up);
        assert!(buttons.down);
        assert!(buttons.button_1);
        assert!(buttons.button_2);
        assert!(!buttons.right);
        assert!(!pause);
    }

    #[test]
    fn gamepad_start_maps_to_sms_pause_nmi() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(6, true).unwrap();

        let (_, pause) = profiles.evaluate(0, DeviceClass::Gamepad, &reading(state));
        assert!(pause);
    }

    #[test]
    fn generic_joystick_third_button_maps_to_sms_pause_nmi() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.buttons.set(34, true).unwrap();

        let (_, pause) = profiles.evaluate(0, DeviceClass::Joystick, &reading(state));
        assert!(pause);
    }

    #[test]
    fn analog_center_noise_does_not_activate_directions() {
        let profiles = SmsInputProfiles::defaults().unwrap();
        let mut state = DeviceState::default();
        state.set_axis(0, 8_000, AxisSemantic::LeftStickX).unwrap();
        state.set_axis(1, -8_000, AxisSemantic::LeftStickY).unwrap();

        let (buttons, _) = profiles.evaluate(0, DeviceClass::Gamepad, &reading(state));
        assert_eq!(buttons, Default::default());
    }

    #[test]
    fn custom_profile_maps_extra_button_and_ors_multiple_controls() {
        let mut profiles = SmsInputProfiles::defaults().unwrap();
        let custom_gamepad = InputProfile::new(
            vec![
                binding(SmsInputAction::Button1, PhysicalControl::Button(8)),
                binding(SmsInputAction::Button1, PhysicalControl::Button(0)),
            ],
            0,
        )
        .unwrap();
        profiles.set_profile(0, DeviceClass::Gamepad, custom_gamepad).unwrap();

        let mut state = DeviceState::default();
        state.buttons.set(8, true).unwrap();
        let (extra_button, _) = profiles.evaluate(0, DeviceClass::Gamepad, &reading(state));
        assert!(extra_button.button_1);
        assert!(!extra_button.button_2);

        let mut state = DeviceState::default();
        state.buttons.set(0, true).unwrap();
        let (standard_button, _) = profiles.evaluate(0, DeviceClass::Gamepad, &reading(state));
        assert!(standard_button.button_1);

        let mut state = DeviceState::default();
        state.buttons.set(8, true).unwrap();
        state.buttons.set(0, true).unwrap();
        let (both_controls, _) = profiles.evaluate(0, DeviceClass::Gamepad, &reading(state));
        assert!(
            both_controls.button_1,
            "bindings for one action must combine with OR"
        );
    }

    #[test]
    fn player_profiles_are_independent_for_the_same_controller_class() {
        let mut profiles = SmsInputProfiles::defaults().unwrap();
        let player_two = InputProfile::new(
            vec![binding(SmsInputAction::Button1, PhysicalControl::Button(7))],
            0,
        ).unwrap();
        profiles.set_profile(1, DeviceClass::Gamepad, player_two).unwrap();

        let mut state = DeviceState::default();
        state.buttons.set(7, true).unwrap();
        let reading = reading(state);
        let (p1, _) = profiles.evaluate(0, DeviceClass::Gamepad, &reading);
        let (p2, _) = profiles.evaluate(1, DeviceClass::Gamepad, &reading);
        assert!(!p1.button_1, "P1 must retain its default mapping");
        assert!(p2.button_1, "P2 must use its own customized mapping");
    }

    #[test]
    fn invalid_player_profile_slot_is_rejected() {
        let mut profiles = SmsInputProfiles::defaults().unwrap();
        let profile = InputProfile::new(Vec::new(), 0).unwrap();
        assert!(profiles.set_profile(2, DeviceClass::Gamepad, profile).is_err());
    }

    #[test]
    fn custom_profile_supports_nonstandard_generic_joystick_axis_order() {
        let mut profiles = SmsInputProfiles::defaults().unwrap();
        let custom_joystick = InputProfile::new(
            vec![binding(
                SmsInputAction::Up,
                PhysicalControl::Axis {
                    index: 19,
                    direction: AxisDirection::Negative,
                    threshold: DEFAULT_STICK_THRESHOLD,
                },
            )],
            0,
        )
        .unwrap();
        profiles.set_profile(0, DeviceClass::Joystick, custom_joystick).unwrap();

        let mut state = DeviceState::default();
        state
            .set_axis(19, -(DEFAULT_STICK_THRESHOLD as i16), AxisSemantic::Unknown)
            .unwrap();
        let (buttons, _) = profiles.evaluate(0, DeviceClass::Joystick, &reading(state));
        assert!(buttons.up, "the configured physical axis must drive SMS Up");
        assert!(!buttons.down);
    }
}
