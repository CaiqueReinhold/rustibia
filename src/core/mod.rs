use bevy::prelude::*;

mod assets;
mod sprite;

pub use crate::core::assets::Appearances;
pub use crate::core::sprite::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, States, Default)]
pub enum State {
    #[default]
    LoadingAssets,
    LoginScreen,
    Connecting,
    InGame,
}
