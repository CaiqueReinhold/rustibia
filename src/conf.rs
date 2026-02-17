pub mod map {
    pub const TILE_SIZE: f32 = 32.0;
    pub const VIEW_TILES_X: f32 = 15.0;
    pub const VIEW_TILES_Y: f32 = 11.0;
    pub const CHUNK_SIZE: u32 = 32;
    pub const CHUNK_LOAD_RADIUS: u32 = 2;
    pub const FLOOR_Z_OFFSET: f32 = 10.0;
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
