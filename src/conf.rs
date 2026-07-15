pub mod map {
    pub const TILE_SIZE: f32 = 32.0;
    pub const VIEW_TILES_X: f32 = 15.0;
    pub const VIEW_TILES_Y: f32 = 11.0;
    pub const TILES_X: usize = 19;
    pub const TILES_Y: usize = 15;
    pub const STACK_MAX_VISIBLE_ITEMS: usize = 8;
    pub const CONTAINER_COORD_FLAG: u16 = 0xFFFF;
    pub const INVENTORY_COORD_FLAG: u16 = 0xFFFE;
    pub const MIN_FLOOR: u8 = 0;
    pub const MAX_FLOOR: u8 = 15;
    pub const BASE_FLOOR: u8 = 7;
}

pub mod z_order {
    pub const FLOOR_Z_MULTIPLIER: f32 = 100.0;
    pub const POSITION_Z_MULTIPLIER: f32 = 0.02;
    pub const AGENT_Z_OFFSET: f32 = 0.013;
    pub const TOP_Z_OFFSET: f32 = 0.015;
    /// Ground and border items render in a separate pass below agents.
    /// -1.0 exceeds the max viewport position delta (~16 tiles × 0.02 = 0.32).
    pub const GROUND_PASS_OFFSET: f32 = -1.0;
}

pub mod agent {
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
    pub const MIN_DRAG_THRESHOLD: f32 = 5.0;
    pub const SEPARATOR_HEIGHT: f32 = 5.0;

    pub mod z_index {
        pub const Z_MAIN_UI: i32 = 10;
        pub const Z_WINDOW: i32 = 11;
        pub const Z_DRAGGING_WINDOW: i32 = 20;
        pub const DRAGGED_ITEM_UI_Z: i32 = 100;
    }

    pub mod ui_colors {
        use bevy::color::Srgba;
        pub const DARK_BORDER_COLOR: Srgba = Srgba::new(0.145098, 0.145098, 0.145098, 1.0);
        pub const LIGHT_BORDER_COLOR: Srgba = Srgba::new(0.4588235, 0.4588235, 0.4588235, 1.0);

        pub const ITEM_SLOT_OUTLINE: Srgba = Srgba::new(0.35, 0.35, 0.35, 1.0);
        pub const ITEM_SLOT_OUTLINE_HOVERED: Srgba = Srgba::new(0.8, 0.8, 0.8, 1.0);

        // pub const FONT_COLOR_TITLE: Srgba = Srgba::new(0.564705, 0.564705, 0.564705, 1.0);
        pub const FONT_COLOR_CONTENT: Srgba = Srgba::new(0.75294, 0.75294, 0.75294, 1.0);
        pub const FONT_COLOR_LOOK_MSG: Srgba = Srgba::rgb(0.0, 0.7372549, 0.0);

        pub const MANA_BAR_COLOR: Srgba = Srgba::new(0.0, 0.0, 0.7, 1.0);
    }

    pub mod chat {
        use bevy::color::Srgba;

        pub const TAB_HEIGHT: f32 = 22.0;
        pub const TAB_MAX_WIDTH: f32 = 90.0;
        pub const INPUT_HEIGHT: f32 = 24.0;
        pub const HISTORY_CAP_DEFAULT: usize = 500;
        pub const LINE_HEIGHT: f32 = 12.;

        pub const UNREAD_TAB_COLOR: Srgba = Srgba::new(0.85, 0.20, 0.20, 1.0);
        pub const TAB_TITLE_COLOR: Srgba = Srgba::new(0.95, 0.95, 0.95, 1.0);
        pub const TAB_TITLE_COLOR_INACTIVE: Srgba = Srgba::new(0.5, 0.5, 0.5, 1.0);
        pub const INPUT_BG_COLOR: Srgba = Srgba::new(0.098, 0.102, 0.106, 1.0);
        pub const INPUT_PLACEHOLDER_COLOR: Srgba = Srgba::new(1.0, 1.0, 1.0, 0.4);

        pub const LOCAL_CHANNEL_NAME: &str = "Local";
        pub const LOCAL_CHANNEL_COLOR: Srgba = Srgba::new(0.94, 0.94, 0.0, 1.0);
    }

    pub mod dialog {
        use bevy::color::Srgba;

        pub const DEFAULT_WIDTH: f32 = 300.0;
        pub const TITLE_BAR_HEIGHT: f32 = 20.0;
        pub const PADDING: f32 = 10.0;
        pub const FIELD_HEIGHT: f32 = 24.0;
        pub const BUTTON_HEIGHT: f32 = 22.0;
        pub const BUTTON_MIN_WIDTH: f32 = 64.0;
        pub const Z_MODAL_BASE: i32 = 100;
        pub const DOUBLE_CLICK_SECS: f32 = 0.4;

        pub const BACKDROP_COLOR: Srgba = Srgba::new(0.0, 0.0, 0.0, 0.4);
        pub const TITLE_BAR_COLOR: Srgba = Srgba::new(0.187, 0.187, 0.187, 1.0);
        pub const BUTTON_COLOR: Srgba = Srgba::new(0.34, 0.34, 0.34, 1.0);
        pub const BUTTON_HOVER_COLOR: Srgba = Srgba::new(0.42, 0.42, 0.42, 1.0);
        pub const FIELD_BG_COLOR: Srgba = Srgba::new(0.098, 0.102, 0.106, 1.0);
        pub const ROW_SELECTED_COLOR: Srgba = Srgba::new(0.25, 0.32, 0.45, 1.0);
    }

    pub mod login {
        use bevy::color::Srgba;

        pub const LOGO_COLOR: Srgba = Srgba::new(0.91, 0.78, 0.38, 1.0);
        pub const LOGO_FONT_SIZE: f32 = 56.0;
        pub const LOGO_TOP_MARGIN: f32 = 40.0;
    }
}

pub mod server {
    pub const TICK_DURATION_MS: u32 = 50;
    pub const SERVER_ADDRESS: &str = "127.0.0.1:5555";
}

pub mod minimap {
    pub const IMAGE_SIZE: u16 = 2048;
    /// Tiles visible per axis at each zoom level (index 0 = most zoomed in).
    pub const ZOOM_LEVELS: [u8; 4] = [20, 40, 80, 160];
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
