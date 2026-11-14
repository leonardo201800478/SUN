//! Host-independent gamepad hotkey chord evaluation.
//!
//! A hotkey may be bound to one physical control or to a chord of up to four
//! controls. Evaluation is allocation-free and does not consume gameplay input;
//! the desktop host decides command priority and edge-triggering.

use crate::{ActionId, ActionState, AxisDirection, DeviceState, PhysicalControl};

const MAX_CHORD_CONTROLS: usize = 4;

/// One hotkey binding. Every control in the chord must be active simultaneously.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyBinding {
    action: ActionId,
    controls: Vec<PhysicalControl>,
}

impl HotkeyBinding {
    /// Creates a single-button or multi-control binding.
    pub fn new(action: ActionId, controls: Vec<PhysicalControl>) -> Result<Self, HotkeyError> {
        if controls.is_empty() {
            return Err(HotkeyError::EmptyChord);
        }
        if controls.len() > MAX_CHORD_CONTROLS {
            return Err(HotkeyError::ChordTooLarge(controls.len()));
        }
        for (index, control) in controls.iter().enumerate() {
            if controls[..index].contains(control) {
                return Err(HotkeyError::DuplicateControl);
            }
            validate_control(*control)?;
        }
        Ok(Self { action, controls })
    }

    pub const fn action(&self) -> ActionId {
        self.action
    }

    pub fn controls(&self) -> &[PhysicalControl] {
        &self.controls
    }
}

/// Immutable set of gamepad hotkeys. Multiple bindings may invoke one action.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotkeyProfile {
    bindings: Vec<HotkeyBinding>,
}

impl HotkeyProfile {
    pub fn new(bindings: Vec<HotkeyBinding>) -> Self {
        Self { bindings }
    }

    /// Evaluates all chords into a caller-owned bitset without allocating.
    ///
    /// A chord is active only while all its controls are active. This method
    /// intentionally does not implement edge detection or consume gameplay
    /// controls; those are host-command routing responsibilities.
    pub fn evaluate(&self, state: &DeviceState, output: &mut ActionState) {
        *output = ActionState::default();
        for binding in &self.bindings {
            if binding
                .controls
                .iter()
                .all(|control| control_is_active(*control, state))
            {
                output.set(binding.action, true);
            }
        }
    }

    pub fn bindings(&self) -> &[HotkeyBinding] {
        &self.bindings
    }
}

fn validate_control(control: PhysicalControl) -> Result<(), HotkeyError> {
    match control {
        PhysicalControl::Button(index) if index as usize >= crate::MAX_BUTTONS => {
            Err(HotkeyError::InvalidButton(index))
        }
        PhysicalControl::Axis {
            index, threshold, ..
        } if index as usize >= crate::MAX_AXES || threshold == 0 || threshold > i16::MAX as u16 => {
            Err(HotkeyError::InvalidAxis { index, threshold })
        }
        PhysicalControl::Hat {
            index,
            direction_mask,
        } if index as usize >= crate::MAX_HATS
            || direction_mask == 0
            || direction_mask & !0x0F != 0 =>
        {
            Err(HotkeyError::InvalidHat {
                index,
                direction_mask,
            })
        }
        _ => Ok(()),
    }
}

fn control_is_active(control: PhysicalControl, state: &DeviceState) -> bool {
    match control {
        PhysicalControl::Button(index) => state.buttons.is_pressed(index as usize),
        PhysicalControl::Axis {
            index,
            direction,
            threshold,
        } => {
            let value = state.axis(index as usize) as i32;
            match direction {
                AxisDirection::Negative => value <= -(threshold as i32),
                AxisDirection::Positive => value >= threshold as i32,
            }
        }
        PhysicalControl::Hat {
            index,
            direction_mask,
        } => state.hat(index as usize) & direction_mask == direction_mask,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyError {
    EmptyChord,
    ChordTooLarge(usize),
    DuplicateControl,
    InvalidButton(u8),
    InvalidAxis { index: u8, threshold: u16 },
    InvalidHat { index: u8, direction_mask: u8 },
}

impl std::fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyChord => write!(f, "hotkey chord cannot be empty"),
            Self::ChordTooLarge(size) => {
                write!(
                    f,
                    "hotkey chord has {size} controls; maximum is {MAX_CHORD_CONTROLS}"
                )
            }
            Self::DuplicateControl => write!(f, "hotkey chord contains a duplicate control"),
            Self::InvalidButton(index) => write!(f, "hotkey button index {index} is out of range"),
            Self::InvalidAxis { index, threshold } => {
                write!(f, "invalid hotkey axis {index} threshold {threshold}")
            }
            Self::InvalidHat {
                index,
                direction_mask,
            } => {
                write!(f, "invalid hotkey hat {index} mask {direction_mask:#04x}")
            }
        }
    }
}

impl std::error::Error for HotkeyError {}

#[cfg(test)]
mod tests {
    use super::{HotkeyBinding, HotkeyError, HotkeyProfile};
    use crate::{ActionId, ActionState, AxisDirection, AxisSemantic, DeviceState, PhysicalControl};

    fn action(value: u8) -> ActionId {
        ActionId::new(value).expect("test action is valid")
    }

    #[test]
    fn single_extra_button_can_trigger_a_host_hotkey() {
        let profile = HotkeyProfile::new(vec![
            HotkeyBinding::new(action(3), vec![PhysicalControl::Button(8)]).unwrap(),
        ]);
        let mut state = DeviceState::default();
        let mut actions = ActionState::default();

        state.buttons.set(8, true).unwrap();
        profile.evaluate(&state, &mut actions);
        assert!(actions.is_active(action(3)));
    }

    #[test]
    fn chord_requires_every_button_and_releases_when_one_is_released() {
        let profile = HotkeyProfile::new(vec![
            HotkeyBinding::new(
                action(4),
                vec![PhysicalControl::Button(6), PhysicalControl::Button(0)],
            )
            .unwrap(),
        ]);
        let mut state = DeviceState::default();
        let mut actions = ActionState::default();

        state.buttons.set(6, true).unwrap();
        profile.evaluate(&state, &mut actions);
        assert!(!actions.is_active(action(4)));

        state.buttons.set(0, true).unwrap();
        profile.evaluate(&state, &mut actions);
        assert!(actions.is_active(action(4)));

        state.buttons.set(0, false).unwrap();
        profile.evaluate(&state, &mut actions);
        assert!(!actions.is_active(action(4)));
    }

    #[test]
    fn chords_support_axis_and_hat_controls() {
        let profile = HotkeyProfile::new(vec![
            HotkeyBinding::new(
                action(5),
                vec![
                    PhysicalControl::Axis {
                        index: 0,
                        direction: AxisDirection::Positive,
                        threshold: 12_000,
                    },
                    PhysicalControl::Hat {
                        index: 0,
                        direction_mask: 0b0011,
                    },
                ],
            )
            .unwrap(),
        ]);
        let mut state = DeviceState::default();
        let mut actions = ActionState::default();
        state.set_axis(0, 12_000, AxisSemantic::Unknown).unwrap();
        state.set_hat(0, 0b0011).unwrap();

        profile.evaluate(&state, &mut actions);
        assert!(actions.is_active(action(5)));
    }

    #[test]
    fn invalid_chords_are_rejected() {
        assert_eq!(
            HotkeyBinding::new(action(0), vec![]).unwrap_err(),
            HotkeyError::EmptyChord
        );
        assert_eq!(
            HotkeyBinding::new(
                action(0),
                vec![PhysicalControl::Button(0), PhysicalControl::Button(0),],
            )
            .unwrap_err(),
            HotkeyError::DuplicateControl
        );
        assert_eq!(
            HotkeyBinding::new(action(0), (0..5).map(PhysicalControl::Button).collect(),)
                .unwrap_err(),
            HotkeyError::ChordTooLarge(5)
        );
    }
}
