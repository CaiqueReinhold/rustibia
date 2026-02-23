use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use futures_lite::future;

use crate::actor::{read_outfits_config, Outfits};
use crate::core::sprite::{read_sprite_sheets, read_sprites_config, Appearances};
use crate::core::{SpriteConfig, SpriteSheet, State};
use crate::map::{read_map_config, Map};

#[derive(Resource)]
pub struct LoadTasks {
    sprite_conf_task: Task<HashMap<u32, Arc<SpriteConfig>>>,
    sprite_sheet_task: Task<HashMap<String, SpriteSheet>>,
    outfits_task: Task<Outfits>,
    map_task: Task<Map>,
}

#[derive(Resource, Debug, Default)]
pub struct GameAssetsLoaded {
    pub map_loaded: bool,
    pub sheets_loaded: bool,
    pub outfits_loaded: bool,
}

pub fn start_load_tasks(mut commands: Commands, assets_server: Res<AssetServer>) {
    let server_clone = assets_server.clone();
    let pool = IoTaskPool::get();
    let sprite_conf_task = pool.spawn(async { read_sprites_config() });
    let sprite_sheet_task = pool.spawn(async move { read_sprite_sheets(&server_clone) });
    let outfits_task = pool.spawn(async { read_outfits_config() });
    let map_task = pool.spawn(async { read_map_config() });

    commands.insert_resource(LoadTasks {
        sprite_conf_task,
        sprite_sheet_task,
        outfits_task,
        map_task,
    });
}

pub fn pool_load_task(
    mut commands: Commands,
    mut tasks: ResMut<LoadTasks>,
    mut game_assets: ResMut<GameAssetsLoaded>,
) {
    info!("pooling load tasks");
    if tasks.sprite_conf_task.is_finished()
        && tasks.sprite_sheet_task.is_finished()
        && !game_assets.sheets_loaded
    {
        let results = (
            future::block_on(future::poll_once(&mut tasks.sprite_conf_task)),
            future::block_on(future::poll_once(&mut tasks.sprite_sheet_task)),
        );
        match results {
            (Some(sprite_conf), Some(sprite_sheet)) => {
                commands.insert_resource(Appearances {
                    sheets: sprite_sheet,
                    sprite_configs: sprite_conf,
                });
                game_assets.sheets_loaded = true;
                info!("Sprites loaded");
            }
            (None, ..) | (.., None) => panic!("Failed to load sprites"),
        }
    }

    if tasks.map_task.is_finished() && !game_assets.map_loaded {
        let result = future::block_on(future::poll_once(&mut tasks.map_task));
        match result {
            Some(map) => {
                commands.insert_resource(map);
                game_assets.map_loaded = true;
                info!("Map loaded");
            }
            None => {
                panic!("Failed to load map");
            }
        }
    }

    if tasks.outfits_task.is_finished() && !game_assets.outfits_loaded {
        let result = future::block_on(future::poll_once(&mut tasks.outfits_task));
        match result {
            Some(outfits) => {
                commands.insert_resource(outfits);
                game_assets.outfits_loaded = true;
                info!("Outfits loaded");
            }
            None => {
                panic!("Failed to load outfits config");
            }
        }
    }
}

pub fn pool_all_assets_loaded(mut commands: Commands, game_assets: Res<GameAssetsLoaded>) {
    if game_assets.map_loaded && game_assets.outfits_loaded && game_assets.sheets_loaded {
        commands.set_state(State::InGame);
        commands.remove_resource::<GameAssetsLoaded>();
        commands.remove_resource::<LoadTasks>();
    }
}
