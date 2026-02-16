use std::collections::HashMap;
use std::sync::Arc;
// use std::time::Duration;

use bevy::prelude::*;

pub mod map_assets;

pub struct AssetsLoaderPlugin;

impl Plugin for AssetsLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_appearances, map_assets::load_map_resource))
            .add_systems(
                Update,
                (check_assets_loaded).run_if(in_state(State::LoadingSprites)),
            );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, States, Default)]
pub enum State {
    #[default]
    LoadingSprites,
    Ready,
}

fn check_assets_loaded(
    mut commands: Commands,
    sheets: Option<Res<AppearanceData>>,
    map: Option<Res<map_assets::WorldMap>>,
) {
    if sheets.is_some() && map.is_some() {
        commands.set_state(State::Ready);
    }
}

// #[derive(Debug, Clone)]
// pub enum LoopType {
//     Infinite,
//     PingPong,
//     Counted,
// }

#[derive(Debug, Clone)]
pub struct AnimationConfig {
    pub phases: Vec<usize>,
    // pub loop_type: LoopType,
    // pub loop_count: usize,
}

#[derive(Debug, Clone)]
pub struct SpriteConfig {
    pub layers: usize,
    pub pattern_x: usize,
    pub pattern_y: usize,
    pub pattern_z: usize,
    pub sprite_indexes: Vec<u32>,
    pub animation: Option<AnimationConfig>,
    // pub bounding_box: Vec<Rect>,
}

impl SpriteConfig {
    pub fn total_animation_phases(&self) -> usize {
        match &self.animation {
            Some(anim) => {
                if anim.phases.len() > 0 {
                    anim.phases.len()
                } else {
                    self.sprite_indexes.len()
                }
            }
            None => 0,
        }
    }

    // pub fn get_phase_duration(&self, phase: usize) -> Duration {
    //     match &self.animation {
    //         Some(anim) => {
    //             if anim.phases.len() > 0 {
    //                 Duration::from_millis(anim.phases[phase] as u64)
    //             } else {
    //                 Duration::from_millis(100)
    //             }
    //         }
    //         None => Duration::from_millis(0),
    //     }
    // }

    // pub fn get_sprite_id(&self, phase: usize, layer: usize, x: usize, y: usize, z: usize) -> u32 {
    //     self.sprite_indexes[(((phase * self.pattern_z + z) * self.pattern_y + y) * self.pattern_x
    //         + x)
    //         * self.layers
    //         + layer]
    // }
}

pub struct SpriteSheet {
    pub size: f32,
    pub grid_size: UVec2,
    pub texture: Handle<Image>,
}

impl SpriteSheet {
    pub fn get_uv(&self, index: u32) -> Rect {
        let cols = self.grid_size.x as f32;
        let rows = self.grid_size.y as f32;

        let tx = index % cols as u32;
        let ty = index / cols as u32;

        let margin_x = 0.5 / (cols * self.size);
        let margin_y = 0.5 / (rows * self.size);

        Rect {
            min: Vec2::new((tx as f32 / cols) + margin_x, (ty as f32 / rows) + margin_y),
            max: Vec2::new(
                ((tx as f32 + 1.0) / cols) - margin_x,
                ((ty as f32 + 1.0) / rows) - margin_y,
            ),
        }
    }
}

#[derive(Resource)]
pub struct AppearanceData {
    pub outfits: HashMap<usize, Arc<SpriteConfig>>,
    pub outfit_sheets: HashMap<usize, Handle<Image>>,
    pub ground_sheets: HashMap<usize, SpriteSheet>,
}

fn load_appearances(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut outfits = HashMap::new();
    let mut sheets = HashMap::new();
    let mut sheets2 = HashMap::new();

    outfits.insert(100, Arc::new(dragon_config()));
    sheets.insert(100, asset_server.load("sprites/100.png"));
    outfits.insert(1921, Arc::new(wings_outfit_config()));
    sheets.insert(1921, asset_server.load("sprites/1921.png"));
    sheets2.insert(
        1,
        SpriteSheet {
            size: 32.0,
            grid_size: UVec2::new(12, 33),
            texture: asset_server.load("sprites/ground.png"),
        },
    );

    commands.insert_resource(AppearanceData {
        outfits,
        outfit_sheets: sheets,
        ground_sheets: sheets2,
    });
}

fn dragon_config() -> SpriteConfig {
    SpriteConfig {
        layers: 1,
        pattern_x: 4,
        pattern_y: 1,
        pattern_z: 1,
        sprite_indexes: (26..58).collect(),
        animation: Some(AnimationConfig {
            phases: vec![100; 8],
        }),
        // bounding_box: Vec::from([
        //     Rect {
        //         min: Vec2::splat(0.0),
        //         max: Vec2::splat(64.0),
        //     },
        //     Rect {
        //         min: Vec2::splat(0.0),
        //         max: Vec2::splat(64.0),
        //     },
        //     Rect {
        //         min: Vec2::splat(0.0),
        //         max: Vec2::splat(64.0),
        //     },
        //     Rect {
        //         min: Vec2::splat(0.0),
        //         max: Vec2::splat(64.0),
        //     },
        // ]),
    }
}

fn wings_outfit_config() -> SpriteConfig {
    SpriteConfig {
        layers: 2,
        pattern_x: 4,
        pattern_y: 3,
        pattern_z: 2,
        sprite_indexes: Vec::<u32>::from([
            96, 97, 98, 99, 100, 101, 102, 103, 104, 0, 10, 0, 105, 0, 106, 0, 107, 108, 109, 110,
            111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 0, 124, 0, 125, 0,
            126, 0, 127, 128, 129, 130, 131, 0, 132, 0, 133, 134, 135, 136, 137, 138, 139, 140,
            141, 0, 142, 0, 143, 0, 144, 0, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155,
            156, 157, 158, 159, 160, 161, 0, 162, 0, 163, 0, 164, 0, 165, 166, 167, 168, 169, 0,
            170, 0, 171, 172, 173, 174, 175, 176, 177, 178, 179, 0, 180, 0, 181, 0, 182, 0, 183,
            184, 185, 186, 17, 18, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 0, 198,
            0, 199, 0, 200, 0, 201, 202, 203, 204, 205, 0, 206, 0, 207, 208, 209, 210, 211, 212,
            213, 214, 215, 0, 216, 0, 217, 0, 218, 0, 219, 220, 221, 222, 223, 224, 225, 0, 226,
            227, 228, 229, 230, 231, 232, 233, 234, 0, 61, 0, 62, 0, 235, 0, 236, 237, 35, 36, 37,
            0, 238, 0, 239, 240, 241, 242, 243, 244, 245, 246, 247, 0, 248, 0, 249, 0, 250, 0, 251,
            252, 253, 254, 255, 256, 257, 0, 258, 259, 260, 261, 262, 263, 264, 265, 266, 0, 267,
            0, 268, 0, 269, 0, 270, 271, 129, 130, 131, 0, 272, 0, 273, 274, 275, 276, 277, 278,
            279, 280, 281, 0, 282, 0, 283, 0, 284, 0, 285, 286, 287, 288, 289, 290, 225, 0, 291,
            292, 293, 294, 295, 296, 297, 298, 299, 0, 300, 0, 301, 0, 302, 0, 303, 304, 167, 168,
            305, 0, 306, 0, 307, 308, 309, 310, 311, 312, 313, 314, 315, 0, 81, 0, 316, 0, 317, 0,
            183, 184, 318, 319, 17, 18, 187, 188, 320, 321, 322, 323, 324, 325, 326, 327, 328, 0,
            329, 0, 330, 0, 331, 0, 332, 333, 203, 204, 334, 0, 335, 0, 336, 337, 338, 339, 340,
            341, 342, 343, 344, 0, 345, 0, 346, 0, 347, 0, 348, 349, 350, 351, 352, 353, 151, 152,
            354, 355, 356, 357, 358, 359, 360, 361, 362, 0, 93, 0, 94, 0, 363, 0, 364, 365, 35, 36,
            37, 0, 366, 0,
        ]),
        animation: Some(AnimationConfig {
            phases: vec![100; 8],
        }),
        // bounding_box: Vec::from([
        //     Rect {
        //         min: Vec2::new(10.0, 21.0),
        //         max: Vec2::new(64.0, 64.0),
        //     },
        //     Rect {
        //         min: Vec2::new(16.0, 12.0),
        //         max: Vec2::new(61.0, 64.0),
        //     },
        //     Rect {
        //         min: Vec2::new(12.0, 16.0),
        //         max: Vec2::new(64.0, 61.0),
        //     },
        //     Rect {
        //         min: Vec2::new(21.0, 10.0),
        //         max: Vec2::new(64.0, 64.0),
        //     },
        // ]),
    }
}
