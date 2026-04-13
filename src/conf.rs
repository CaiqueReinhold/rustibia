pub mod map {
    pub const TILE_SIZE: f32 = 32.0;
    pub const VIEW_TILES_X: f32 = 15.0;
    pub const VIEW_TILES_Y: f32 = 11.0;
    pub const TILES_X: usize = 19;
    pub const TILES_Y: usize = 15;
    pub const STACK_MAX_VISIBLE_ITEMS: usize = 8;
    pub const CONTAINER_COORD_FLAG: u32 = 0xFFFFFFFF;
    pub const INVENTORY_COORD_FLAG: u32 = 0xFFFFFFFE;
}

pub mod z_order {
    pub const FLOOR_Z_MULTIPLIER: f32 = 100.0;
    pub const POSITION_Z_MULTIPLIER: f32 = 0.02;
    pub const ACTOR_Z_OFFSET: f32 = 0.01;
    pub const TOP_Z_OFFSET: f32 = 0.015;
}

pub mod actor {
    // pub const ADDONS_NONE: u8 = 0;
    pub const ADDON_1_FLAG: u8 = 0b1;
    pub const ADDON_2_FLAG: u8 = 0b10;
    pub const SPEED_PARAM_A: f32 = 857.36;
    pub const SPEED_PARAM_B: f32 = 261.29;
    pub const SPEED_PARAM_C: f32 = -4795.009;
    pub const HUD_BAR_WIDTH: f32 = 30.0;
    pub const HUD_BAR_HEIGHT: f32 = 4.0;
}

pub mod viewport {
    use super::map;
    pub const GAME_VIEW_WIDTH: f32 = map::VIEW_TILES_X * map::TILE_SIZE;
    pub const GAME_VIEW_HEIGHT: f32 = map::VIEW_TILES_Y * map::TILE_SIZE;
    // pub const GAME_VIEW_MIN_SIZE: f32 = 400.0;
}

pub mod ui {
    pub const TOP_BAR_HEIGHT: f32 = 50.0;
    pub const SIDE_PANEL_WIDTH: f32 = 180.0;
    pub const CHAT_BOX_HEIGHT: f32 = 170.0;
    pub const UI_ITEM_SIZE: f32 = 32.0;
    pub const LOOT_CONTAINER_DEFAULT_HEIGHT: usize = 40;
    pub const INVENTORY_HEIGHT: f32 = 170.0;
    pub const ITEM_SLOT_SIZE: f32 = 36.0;
    pub const UI_BAR_HEIGHT: f32 = 20.0;

    pub mod z_index {
        pub const Z_WINDOW: i32 = 10;
        pub const Z_DRAGGING_WINDOW: i32 = 20;
        pub const DRAGGED_ITEM_UI_Z: i32 = 100;
        pub const Z_AGENT_HUD: i32 = 10;
    }

    pub mod ui_colors {
        use bevy::color::Srgba;
        pub const DARK_BORDER_COLOR: Srgba = Srgba::new(0.145098, 0.145098, 0.145098, 1.0);
        pub const LIGHT_BORDER_COLOR: Srgba = Srgba::new(0.4588235, 0.4588235, 0.4588235, 1.0);

        pub const ITEM_SLOT_OUTLINE: Srgba = Srgba::new(0.35, 0.35, 0.35, 1.0);
        pub const ITEM_SLOT_OUTLINE_HOVERED: Srgba = Srgba::new(0.8, 0.8, 0.8, 1.0);

        // pub const FONT_COLOR_TITLE: Srgba = Srgba::new(0.564705, 0.564705, 0.564705, 1.0);
        pub const FONT_COLOR_CONTENT: Srgba = Srgba::new(0.75294, 0.75294, 0.75294, 1.0);

        pub const MANA_BAR_COLOR: Srgba = Srgba::new(0.0, 0.0, 0.7, 1.0);
    }
}

pub mod server {
    pub const TICK_DURATION_MS: u32 = 50;
    pub const SERVER_ADDRESS: &str = "127.0.0.1:5555";
}

pub mod minimap {
    pub const IMAGE_SIZE: u32 = 2048;
    /// Tiles visible per axis at each zoom level (index 0 = most zoomed in).
    pub const ZOOM_LEVELS: [u32; 5] = [20, 40, 80, 160, 320];
    pub const DEFAULT_ZOOM: usize = 2; // 80×80 tiles
}

pub mod paths {
    use std::path::PathBuf;

    /// Returns the root data directory for persistent game data.
    ///
    /// - Linux:   `~/.local/share/Rustibia`
    /// - Windows: `%APPDATA%\Rustibia`
    pub fn data_dir() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("Rustibia")
        }
        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(appdata).join("Rustibia")
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            PathBuf::from("data")
        }
    }
}
