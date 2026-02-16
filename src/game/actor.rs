use std::sync::Arc;

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

use crate::conf::map::TILE_SIZE;
use crate::data::{AppearanceData, SpriteConfig, State};

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<ActorMaterial>::default())
            .add_systems(Update, (setup_mesh).run_if(in_state(State::Ready)));
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub struct ActorMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,

    #[uniform(2)]
    pub params: ActorParams,

    #[storage(3, read_only)]
    pub sprite_indexes: Handle<ShaderStorageBuffer>,
}

impl Material2d for ActorMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/actor.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(ShaderType, Clone, Copy, Debug, Default)]
pub struct ActorParams {
    pub atlas_grid: Vec2,

    // layout
    pub pattern_x: u32,
    pub pattern_y: u32,
    pub pattern_z: u32,
    pub layers: u32,

    // dynamic state
    pub direction: u32, // x
    pub addon: u32,     // y
    pub mounted: u32,   // z
    pub _pad0: u32,
    // animation
    pub phase_count: u32,
    pub _pad1: Vec3,
    pub phase_duration: f32,
    pub time_offset: f32,
    pub _pad2: Vec2,

    // recolor
    pub color0: Vec4,
    pub color1: Vec4,
    pub color2: Vec4,
    pub color3: Vec4,
}

#[derive(Component, Debug)]
pub struct Actor {
    sprite_config: Arc<SpriteConfig>,
    direction: usize,
    addons: usize,
    mounted: usize,
}

fn spawn_actor_mesh(
    pos: Vec2,
    materials: &mut Assets<ActorMaterial>,
    meshes: &mut Assets<Mesh>,
    buffers: &mut Assets<ShaderStorageBuffer>,
    texture_handle: &Handle<Image>,
    time: &Time,
    actor: Actor,
) -> impl Bundle {
    let world_pos = Vec3::new(
        pos.x as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        pos.y as f32 * TILE_SIZE + TILE_SIZE / 2.0,
        pos.x * 0.01 + pos.y * 0.01,
    );
    let params = ActorParams {
        atlas_grid: Vec2::new(12.0, 31.0),

        pattern_x: actor.sprite_config.pattern_x as u32,
        pattern_y: actor.sprite_config.pattern_y as u32,
        pattern_z: actor.sprite_config.pattern_z as u32,
        layers: actor.sprite_config.layers as u32,

        direction: actor.direction as u32,
        addon: actor.addons as u32,
        mounted: actor.mounted as u32,

        phase_count: actor.sprite_config.total_animation_phases() as u32,
        phase_duration: 0.1,
        time_offset: time.elapsed_secs_wrapped(),

        color0: Srgba::WHITE.to_vec4(),
        color1: Srgba::BLUE.to_vec4(),
        color2: Srgba::RED.to_vec4(),
        color3: Srgba::BLACK.to_vec4(),
        _pad0: 0,
        _pad1: Vec3::ZERO,
        _pad2: Vec2::ZERO,
    };

    let material = materials.add(ActorMaterial {
        texture: texture_handle.clone(),
        params,
        sprite_indexes: buffers.add(ShaderStorageBuffer::from(
            actor.sprite_config.sprite_indexes.as_slice() as &[u32],
        )),
    });

    (
        actor,
        Mesh2d(meshes.add(Mesh::from(Rectangle::new(64.0, 64.0)))),
        MeshMaterial2d(material),
        Transform::from_xyz(world_pos.x, world_pos.y, world_pos.z),
    )
}

fn setup_mesh(
    actors_q: Query<&Actor>,
    mut materials: ResMut<Assets<ActorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    appearances: Res<AppearanceData>,
    mut commands: Commands,
    time: Res<Time>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    if !actors_q.is_empty() {
        return;
    }

    let texture_handle = appearances.outfit_sheets.get(&1921).unwrap().clone();
    let sprite_config = appearances.outfits.get(&1921).unwrap().clone();

    // for x in 0..15 {
    //     for y in 0..11 {
    //         commands.spawn(spawn_actor_mesh(
    //             Vec2::new((x - 7) as f32, (5 - y) as f32),
    //             &mut materials,
    //             &mut meshes,
    //             &mut buffers,
    //             &texture_handle,
    //             &time,
    //             Actor {
    //                 sprite_config: sprite_config.clone(),
    //                 direction: 0,
    //                 addons: 0,
    //                 mounted: 0,
    //             },
    //         ));
    //     }
    // }
}
