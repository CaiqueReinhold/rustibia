use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::network::events::{ExperienceChanged, PlayerSkills, SkillChanged};

/// Ids assigned by the server, which stores them in `player_skills.skill_type`
/// as well as sending them. Explicit, so reordering the variants cannot change
/// one; the matching assertion is in the server's `entities/skills.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillType {
    Level = 0,
    Axe = 1,
    Club = 2,
    Sword = 3,
    Distance = 4,
    Magic = 5,
    Shielding = 6,
}

impl SkillType {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(SkillType::Level),
            1 => Some(SkillType::Axe),
            2 => Some(SkillType::Club),
            3 => Some(SkillType::Sword),
            4 => Some(SkillType::Distance),
            5 => Some(SkillType::Magic),
            6 => Some(SkillType::Shielding),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SkillType::Level => "Level",
            SkillType::Axe => "Axe",
            SkillType::Club => "Club",
            SkillType::Sword => "Sword",
            SkillType::Distance => "Distance",
            SkillType::Magic => "Magic Level",
            SkillType::Shielding => "Shielding",
        }
    }
}

/// Top to bottom in the window. Not id order: this is what a player expects to
/// read, and a skill missing from the server's snapshot draws no row at all.
pub const DISPLAY_ORDER: [SkillType; 7] = [
    SkillType::Level,
    SkillType::Magic,
    SkillType::Sword,
    SkillType::Axe,
    SkillType::Club,
    SkillType::Distance,
    SkillType::Shielding,
];

/// `percent_bp` is hundredths of a percent, `0..=10_000`.
#[derive(Clone, Copy, Debug)]
pub struct SkillProgress {
    pub level: u16,
    pub percent_bp: u16,
}

impl SkillProgress {
    pub fn ratio(&self) -> f32 {
        self.percent_bp as f32 / 10_000.0
    }
}

#[derive(Resource, Default)]
pub struct SkillsState {
    pub experience: u64,
    pub skills: HashMap<SkillType, SkillProgress>,
}

pub fn on_player_skills(event: On<PlayerSkills>, mut state: ResMut<SkillsState>) {
    state.experience = event.experience;
    state.skills = event.skills.iter().copied().collect();
}

pub fn on_skill_changed(event: On<SkillChanged>, mut state: ResMut<SkillsState>) {
    state.skills.insert(event.skill, event.progress);
}

pub fn on_experience_changed(event: On<ExperienceChanged>, mut state: ResMut<SkillsState>) {
    state.experience = event.experience;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_with_state() -> World {
        let mut world = World::new();
        world.init_resource::<SkillsState>();
        world.add_observer(on_player_skills);
        world.add_observer(on_skill_changed);
        world.add_observer(on_experience_changed);
        world
    }

    #[test]
    fn a_snapshot_replaces_everything() {
        let mut world = world_with_state();
        world.resource_mut::<SkillsState>().skills.insert(
            SkillType::Axe,
            SkillProgress {
                level: 99,
                percent_bp: 1,
            },
        );

        world.trigger(PlayerSkills {
            experience: 4231,
            skills: vec![(
                SkillType::Sword,
                SkillProgress {
                    level: 12,
                    percent_bp: 4909,
                },
            )],
        });

        let state = world.resource::<SkillsState>();
        assert_eq!(state.experience, 4231);
        assert_eq!(state.skills.len(), 1);
        assert_eq!(state.skills[&SkillType::Sword].level, 12);
    }

    #[test]
    fn a_delta_patches_one_skill_and_leaves_the_others() {
        let mut world = world_with_state();
        world.trigger(PlayerSkills {
            experience: 4231,
            skills: vec![
                (
                    SkillType::Sword,
                    SkillProgress {
                        level: 12,
                        percent_bp: 4909,
                    },
                ),
                (
                    SkillType::Axe,
                    SkillProgress {
                        level: 10,
                        percent_bp: 0,
                    },
                ),
            ],
        });

        world.trigger(SkillChanged {
            skill: SkillType::Sword,
            progress: SkillProgress {
                level: 13,
                percent_bp: 0,
            },
        });

        let state = world.resource::<SkillsState>();
        assert_eq!(state.skills[&SkillType::Sword].level, 13);
        assert_eq!(state.skills[&SkillType::Axe].level, 10);
        assert_eq!(
            state.experience, 4231,
            "a skill delta carries no experience"
        );
    }

    #[test]
    fn experience_arrives_on_its_own_message() {
        let mut world = world_with_state();

        world.trigger(ExperienceChanged { experience: 4255 });

        assert_eq!(world.resource::<SkillsState>().experience, 4255);
    }

    /// The server repeats these ids and nothing links the two — separate
    /// repositories, no shared crate. The matching assertion lives in the
    /// server's `entities/skills.rs`. A divergence is silent: every id still
    /// decodes to some skill, so the bars are simply labelled wrong.
    #[test]
    fn the_wire_ids_match_the_server() {
        assert_eq!(SkillType::Level as u8, 0);
        assert_eq!(SkillType::Axe as u8, 1);
        assert_eq!(SkillType::Club as u8, 2);
        assert_eq!(SkillType::Sword as u8, 3);
        assert_eq!(SkillType::Distance as u8, 4);
        assert_eq!(SkillType::Magic as u8, 5);
        assert_eq!(SkillType::Shielding as u8, 6);
    }

    #[test]
    fn every_id_round_trips_and_unknown_ids_are_rejected() {
        for skill in DISPLAY_ORDER {
            assert_eq!(SkillType::from_id(skill as u8), Some(skill));
        }
        assert_eq!(SkillType::from_id(7), None);
    }

    #[test]
    fn a_ratio_is_the_bar_width() {
        assert_eq!(
            SkillProgress {
                level: 1,
                percent_bp: 0
            }
            .ratio(),
            0.0
        );
        assert_eq!(
            SkillProgress {
                level: 1,
                percent_bp: 5_000
            }
            .ratio(),
            0.5
        );
        assert_eq!(
            SkillProgress {
                level: 1,
                percent_bp: 10_000
            }
            .ratio(),
            1.0
        );
    }
}
