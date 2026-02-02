use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;

pub fn spawn_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    let map_handle: Handle<TiledMapAsset> = asset_server.load("maps/map.tmx");

    commands.spawn((
        TiledMap(map_handle),
        TilemapAnchor::Center,
        Transform::from_xyz(0.0, 0.0, 1.0),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
    ));
}

pub fn debug_print_map_loaded(
    maps: Query<&TiledMap, Added<TiledMap>>,
    assets: Res<Assets<TiledMapAsset>>,
) {
    for map in maps.iter() {
        if let Some(map_asset) = assets.get(&map.0) {
            info!("Map loaded with size: {:?}", map_asset.map);
        }
    }
}
