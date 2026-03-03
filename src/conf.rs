pub mod map {
    pub const TILE_SIZE: f32 = 32.0;
    pub const VIEW_TILES_X: f32 = 15.0;
    pub const VIEW_TILES_Y: f32 = 11.0;
    pub const CHUNK_SIZE: u32 = 32;
    pub const CHUNK_LOAD_RADIUS: u32 = 2;
}

pub mod z_order {
    pub const FLOOR_Z_MULTIPLIER: f32 = 10.0;
    pub const ACTOR_Z_OFFSET: f32 = 1.0;
}

pub mod actor {
    // pub const ADDONS_NONE: u32 = 0;
    // pub const ADDON_1_FLAG: u32 = 0b1;
    // pub const ADDON_2_FLAG: u32 = 0b10;
    pub const SPEED_PARAM_A: f32 = 857.36;
    pub const SPEED_PARAM_B: f32 = 261.29;
    pub const SPEED_PARAM_C: f32 = -4795.009;
}

pub mod viewport {
    use super::map;
    pub const GAME_VIEW_WIDTH: f32 = map::VIEW_TILES_X * map::TILE_SIZE;
    pub const GAME_VIEW_HEIGHT: f32 = map::VIEW_TILES_Y * map::TILE_SIZE;
    pub const ASPECT_RATIO: f32 = GAME_VIEW_WIDTH / GAME_VIEW_HEIGHT;
}

pub mod ui {
    pub const TOP_BAR_HEIGHT: f32 = 80.0;
    pub const SIDE_PANEL_WIDTH: f32 = 220.0;
    pub const CHAT_BOX_HEIGHT: f32 = 220.0;
}

pub mod server {
    pub const TICK_DURATION_MS: u32 = 50;
}
