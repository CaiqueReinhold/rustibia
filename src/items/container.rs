use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    conf::ui::{ui_colors, LOOT_CONTAINER_DEFAULT_HEIGHT},
    core::{Appearances, ItemConfigs},
    game_ui::{AddUIWindow, CloseUIWindow, GameUiAssets, ReplaceUIWindowContent, UiWindowRef},
    items::{ui_item::spawn_ui_item, Item, ItemId, OpenParentContainer},
    network::{
        events::{ContainerClosed, OpenContainer, UpdateContainer},
        ClientMessage, SendMessage,
    },
    player::{MouseHoverState, PendingUseAck},
};

const SLOT_SIZE: f32 = 32.0;
const SLOT_MARGIN: f32 = 1.0;

pub type ContainerId = u16;

#[derive(Resource)]
pub struct PreventContainerCloseEvent {
    container_id: ContainerId,
}

#[derive(Component)]
pub struct LootContainerUI {
    pub container_id: ContainerId,
    pub capacity: usize,
    pub items: Vec<Arc<Item>>,
}

impl LootContainerUI {
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.capacity
    }
}

#[derive(Component)]
pub struct ContainerSlot {
    index: usize,
}

fn on_enter_slot(
    event: On<Pointer<Over>>,
    mut commands: Commands,
    mut hover_state: ResMut<MouseHoverState>,
    container_q: Query<(&ChildOf, &ContainerSlot)>,
) {
    commands.entity(event.entity).insert(Outline {
        width: Val::Px(1.0),
        offset: Val::Px(0.0),
        color: Color::from(ui_colors::ITEM_SLOT_OUTLINE_HOVERED),
    });
    let Ok((container, slot)) = container_q.get(event.entity) else {
        return;
    };
    hover_state.container = Some(container.parent());
    hover_state.container_slot = Some(slot.index);
}

fn on_leave_slot(
    event: On<Pointer<Out>>,
    mut commands: Commands,
    mut hover_state: ResMut<MouseHoverState>,
) {
    commands.entity(event.entity).insert(Outline {
        width: Val::Px(1.0),
        offset: Val::Px(0.0),
        color: Color::from(ui_colors::ITEM_SLOT_OUTLINE),
    });
    hover_state.container = None;
    hover_state.container_slot = None;
}

fn as_item_vec(items: &[Option<(ItemId, u8)>], configs: &ItemConfigs) -> Vec<Arc<Item>> {
    let mut items_vec = Vec::new();
    for it in items.iter() {
        if let Some((id, amount)) = it {
            let item = Arc::new(Item::new(
                configs.items.get(id).unwrap().clone(),
                *amount as u32,
            ));
            items_vec.push(item);
        } else {
            break;
        }
    }
    items_vec
}

pub fn on_open_container(
    event: On<OpenContainer>,
    mut commands: Commands,
    configs: Res<ItemConfigs>,
    ui_assets: Res<GameUiAssets>,
    loot_container_q: Query<(&LootContainerUI, &UiWindowRef)>,
    pending_ack: Option<Res<PendingUseAck>>,
) {
    let container = loot_container_q
        .iter()
        .find(|c| c.0.container_id == event.container_id);
    if let Some((_, window_ref)) = container {
        if let Some(ack) = &pending_ack {
            if ack.target_window_id.is_some() && ack.target_window_id != Some(window_ref.window_id)
            {
                commands.insert_resource(PreventContainerCloseEvent {
                    container_id: event.container_id,
                });
                commands.trigger(CloseUIWindow {
                    window_id: window_ref.window_id,
                });
            }
        } else {
            return;
        }
    }

    let container = LootContainerUI {
        container_id: event.container_id,
        capacity: event.capacity as usize,
        items: as_item_vec(&event.items, &configs),
    };
    let grid = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                padding: UiRect::new(Val::Px(5.0), Val::Px(3.0), Val::Px(3.0), Val::Px(3.0)),
                row_gap: Val::Px(3.0),
                column_gap: Val::Px(3.0),
                ..default()
            },
            Transform::default(),
        ))
        .id();

    for i in 0..container.capacity {
        let mut slot_cmds = commands.spawn((
            ContainerSlot { index: i },
            Node {
                width: Val::Px(SLOT_SIZE),
                height: Val::Px(SLOT_SIZE),
                margin: UiRect::all(Val::Px(SLOT_MARGIN)),
                ..default()
            },
            Transform::default(),
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 1.0)),
            Outline {
                width: Val::Px(1.0),
                offset: Val::Px(0.0),
                color: Color::from(ui_colors::ITEM_SLOT_OUTLINE),
            },
        ));
        slot_cmds.observe(on_enter_slot);
        slot_cmds.observe(on_leave_slot);

        let slot_id = slot_cmds.id();
        commands.entity(grid).add_child(slot_id);
    }

    commands.entity(grid).insert(container);

    let container_id = event.container_id;
    let custom_buttons = if event.has_parent {
        let button = commands
            .spawn((
                Node {
                    width: Val::Px(10.0),
                    height: Val::Px(10.0),
                    ..default()
                },
                ImageNode {
                    image: ui_assets.window.parent_container.clone(),
                    ..default()
                },
            ))
            .observe(move |mut e: On<Pointer<Click>>, mut commands: Commands| {
                e.propagate(false);
                info!(
                    "Open parent container button clicked for container {}",
                    container_id
                );
                commands.trigger(OpenParentContainer { container_id });
            })
            .observe(|mut e: On<Pointer<DragStart>>| {
                e.propagate(false);
            })
            .id();
        vec![button]
    } else {
        Vec::new()
    };

    if let Some(window_id) = pending_ack.as_ref().and_then(|ack| ack.target_window_id) {
        commands.trigger(ReplaceUIWindowContent {
            window_id,
            content: grid,
            title: event.title.clone(),
            custom_buttons,
        });
    } else {
        commands.trigger(AddUIWindow {
            content: grid,
            default_height: LOOT_CONTAINER_DEFAULT_HEIGHT,
            title: event.title.clone(),
            custom_buttons,
        });
    }

    if pending_ack.is_some() {
        commands.remove_resource::<PendingUseAck>();
    }
}

pub fn on_open_parent_container(
    event: On<OpenParentContainer>,
    mut commands: Commands,
    loot_container_q: Query<(&LootContainerUI, &UiWindowRef)>,
) {
    info!(
        "Requesting to open parent container for container {}",
        event.container_id
    );
    for (container_ui, window_ref) in loot_container_q.iter() {
        info!(
            "Checking container {} with ID {:?}",
            container_ui.container_id, window_ref.window_id
        );
    }
    let Some((_, window_ref)) = loot_container_q
        .iter()
        .find(|c| c.0.container_id == event.container_id)
    else {
        info!("Container not found for ID: {}", event.container_id);
        return;
    };

    commands.insert_resource(PendingUseAck {
        target_window_id: Some(window_ref.window_id),
    });
    commands.trigger(SendMessage {
        msg: ClientMessage::OpenParentContainer {
            container_id: event.container_id,
        },
    });
}

pub fn on_update_container(
    event: On<UpdateContainer>,
    configs: Res<ItemConfigs>,
    mut loot_container_q: Query<&mut LootContainerUI>,
) {
    for mut container in loot_container_q.iter_mut() {
        if container.container_id == event.container_id {
            container.items = as_item_vec(&event.items, &configs);
            break;
        }
    }
}

pub fn on_container_closed_by_server(
    event: On<ContainerClosed>,
    mut commands: Commands,
    loot_container_q: Query<(&LootContainerUI, &UiWindowRef)>,
) {
    for (container_ui, window_ref) in loot_container_q.iter() {
        if container_ui.container_id == event.container_id {
            commands.trigger(CloseUIWindow {
                window_id: window_ref.window_id,
            });
            break;
        }
    }
}

pub fn on_container_ui_closed(
    event: On<Remove, LootContainerUI>,
    mut commands: Commands,
    loot_container_q: Query<&LootContainerUI>,
    prevent_close: Option<Res<PreventContainerCloseEvent>>,
) {
    let Ok(loot_container) = loot_container_q.get(event.entity) else {
        return;
    };

    if let Some(prevent_close) = prevent_close.as_ref() {
        if prevent_close.container_id == loot_container.container_id {
            commands.remove_resource::<PreventContainerCloseEvent>();
            return;
        }
    }

    commands.trigger(SendMessage {
        msg: ClientMessage::CloseContainer {
            container_id: loot_container.container_id,
        },
    });
}

pub fn container_content_changed(
    mut commands: Commands,
    container_q: Query<(&LootContainerUI, &Children), Changed<LootContainerUI>>,
    appearances: Res<Appearances>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    for (container, children) in container_q {
        for (i, child) in children.iter().enumerate() {
            commands.entity(child).despawn_children();
            if let Some(item) = container.items.get(i) {
                commands.entity(child).with_child(spawn_ui_item(
                    item,
                    &appearances,
                    &mut texture_atlases,
                    &Vec2::ZERO,
                ));
            }
        }
    }
}
