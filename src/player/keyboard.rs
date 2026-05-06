use std::time::Duration;

use bevy::prelude::*;

use crate::{
    agent::{FacingDirection, WalkingDirection},
    player::movement::{ChangePlayerDirection, MovePlayer},
};

#[derive(Clone, Debug)]
pub enum PlayerAction {
    Move(WalkingDirection),
    ChangeDirection(FacingDirection),
}

#[derive(Clone, Debug)]
pub struct KeyCombo {
    /// Any of these being just_pressed activates the combo
    pub keys: Vec<KeyCode>,
    /// All of these must be held (empty = no modifier required)
    pub modifiers: Vec<KeyCode>,
}

impl KeyCombo {
    pub fn single(key: KeyCode) -> Self {
        Self {
            keys: vec![key],
            modifiers: vec![],
        }
    }
    pub fn any(keys: Vec<KeyCode>) -> Self {
        Self {
            keys,
            modifiers: vec![],
        }
    }
    pub fn modified(modifier: KeyCode, key: KeyCode) -> Self {
        Self {
            keys: vec![key],
            modifiers: vec![modifier],
        }
    }
    pub fn matches(&self, key: &KeyCode, modifiers: &Vec<&KeyCode>) -> bool {
        self.modifiers.iter().all(|m| modifiers.contains(&m)) && self.keys.contains(key)
    }
}

#[derive(Resource)]
pub struct Keybinds {
    pub binds: Vec<(KeyCombo, PlayerAction)>,
}

impl Default for Keybinds {
    fn default() -> Self {
        use KeyCode::*;
        Self {
            binds: vec![
                // Shift combos must come before bare keys
                (
                    KeyCombo::modified(ShiftLeft, KeyW),
                    PlayerAction::ChangeDirection(FacingDirection::North),
                ),
                (
                    KeyCombo::modified(ShiftLeft, KeyD),
                    PlayerAction::ChangeDirection(FacingDirection::East),
                ),
                (
                    KeyCombo::modified(ShiftLeft, KeyS),
                    PlayerAction::ChangeDirection(FacingDirection::South),
                ),
                (
                    KeyCombo::modified(ShiftLeft, KeyA),
                    PlayerAction::ChangeDirection(FacingDirection::West),
                ),
                (
                    KeyCombo::any(vec![KeyW, ArrowUp]),
                    PlayerAction::Move(WalkingDirection::North),
                ),
                (
                    KeyCombo::any(vec![KeyD, ArrowRight]),
                    PlayerAction::Move(WalkingDirection::East),
                ),
                (
                    KeyCombo::any(vec![KeyS, ArrowDown]),
                    PlayerAction::Move(WalkingDirection::South),
                ),
                (
                    KeyCombo::any(vec![KeyA, ArrowLeft]),
                    PlayerAction::Move(WalkingDirection::West),
                ),
                (
                    KeyCombo::single(KeyQ),
                    PlayerAction::Move(WalkingDirection::NorthWest),
                ),
                (
                    KeyCombo::single(KeyE),
                    PlayerAction::Move(WalkingDirection::NorthEast),
                ),
                (
                    KeyCombo::single(KeyZ),
                    PlayerAction::Move(WalkingDirection::SouthWest),
                ),
                (
                    KeyCombo::single(KeyC),
                    PlayerAction::Move(WalkingDirection::SouthEast),
                ),
            ],
        }
    }
}

#[derive(Resource, Debug)]
pub struct KeyRepeatState {
    pressed_key: Option<KeyCode>,
    timer: Timer,
}

pub fn init_repeat_state(mut commands: Commands) {
    commands.insert_resource(KeyRepeatState {
        pressed_key: None,
        timer: Timer::new(Duration::from_millis(200), TimerMode::Repeating),
    });
}

pub fn read_player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut key_repeat: ResMut<KeyRepeatState>,
    mut commands: Commands,
    time: Res<Time>,
) {
    key_repeat.timer.tick(time.delta());
    let is_modifier = |k: &&KeyCode| {
        matches!(
            k,
            KeyCode::AltLeft
                | KeyCode::AltRight
                | KeyCode::ShiftLeft
                | KeyCode::ShiftRight
                | KeyCode::ControlLeft
                | KeyCode::ControlRight
        )
    };
    let just_pressed_key = keyboard.get_just_pressed().find(|k| !is_modifier(k));

    let mut pressed = None;
    if let Some(key) = just_pressed_key {
        pressed = Some(*key);
    } else if let Some(key) = key_repeat.pressed_key
        && keyboard.pressed(key)
    {
        pressed = Some(key);
    }

    if key_repeat.pressed_key.is_some()
        && key_repeat.pressed_key == pressed
        && !key_repeat.timer.just_finished()
    {
        return;
    }

    if key_repeat.pressed_key != pressed {
        key_repeat.pressed_key = pressed;
        key_repeat.timer.reset();
    }

    if let Some(key) = pressed {
        let modifiers: Vec<&KeyCode> = keyboard.get_pressed().filter(is_modifier).collect();
        for (combo, action) in &keybinds.binds {
            if combo.matches(&key, &modifiers) {
                route_action(action, &mut commands);
                break;
            }
        }
    }
}

fn route_action(action: &PlayerAction, commands: &mut Commands) {
    match action {
        PlayerAction::Move(dir) => commands.trigger(MovePlayer { direction: *dir }),
        PlayerAction::ChangeDirection(dir) => {
            commands.trigger(ChangePlayerDirection { direction: *dir })
        }
    }
}

pub fn cancel_targeting_on_escape(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    targeting: Option<Res<crate::player::UseWithTargetingState>>,
) {
    if targeting.is_some() && keyboard.just_pressed(KeyCode::Escape) {
        commands.remove_resource::<crate::player::UseWithTargetingState>();
    }
}
