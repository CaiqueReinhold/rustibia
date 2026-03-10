use bevy::prelude::*;

mod interaction;

pub use interaction::{ItemDragOrigin, MouseHoverState};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<interaction::MouseHoverState>()
            .add_systems(PreUpdate, interaction::update_hover_state)
            .add_observer(interaction::attach_observers);
    }
}
