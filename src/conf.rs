pub mod map {
    pub const TILE_SIZE: f32 = 32.0;
    pub const VIEW_TILES_X: f32 = 15.0;
    pub const VIEW_TILES_Y: f32 = 11.0;
    pub const TILES_X: usize = 19;
    pub const TILES_Y: usize = 15;
    pub const STACK_MAX_VISIBLE_ITEMS: usize = 8;
}

pub mod z_order {
    pub const FLOOR_Z_MULTIPLIER: f32 = 100.0;
    pub const POSITION_Z_MULTIPLIER: f32 = 0.02;
    pub const ACTOR_Z_OFFSET: f32 = 0.01;
    pub const TOP_Z_OFFSET: f32 = 0.015;
}

pub mod actor {
    // pub const ADDONS_NONE: u32 = 0;
    pub const ADDON_1_FLAG: u32 = 0b1;
    pub const ADDON_2_FLAG: u32 = 0b10;
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
    pub const SIDE_PANEL_WIDTH: f32 = 180.0;
    pub const CHAT_BOX_HEIGHT: f32 = 220.0;
    pub const UI_ITEM_SIZE: f32 = 32.0;

    pub mod z_index {
        pub const DRAGGED_ITEM_UI_Z: i32 = 100;
    }

    pub mod ui_colors {
        use bevy::color::Srgba;

        pub const ITEM_SLOT_OUTLINE: Srgba = Srgba::new(0.35, 0.35, 0.35, 1.0);
        pub const ITEM_SLOT_OUTLINE_HOVERED: Srgba = Srgba::new(0.8, 0.8, 0.8, 1.0);
    }
}

pub mod server {
    pub const TICK_DURATION_MS: u32 = 50;
    pub const SERVER_ADDRESS: &str = "127.0.0.1:5555";
}
