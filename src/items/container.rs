use std::sync::Arc;

use bevy::prelude::*;

use crate::{
    conf::ui::ui_colors,
    core::{Appearances, ItemConfigs},
    items::{ui_item::spawn_ui_item, Item, ItemId},
    main_ui::{AddUIWindow, CloseUIWindow, UiWindowRef},
    network::{
        events::{ContainerClosed, OpenContainer, UpdateContainer},
        ClientMessage, SendMessage,
    },
    player::MouseHoverState,
};

const SLOT_SIZE: f32 = 32.0;
const SLOT_MARGIN: f32 = 1.0;

pub type ContainerId = u16;

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
    loot_container_q: Query<&LootContainerUI>,
) {
    for container in loot_container_q.iter() {
        if container.container_id == event.container_id {
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

    commands.trigger(AddUIWindow {
        content: grid,
        default_height: 40,
        title: event.title.clone(),
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

pub fn on_container_closed(
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
) {
    let Ok(loot_container) = loot_container_q.get(event.entity) else {
        return;
    };

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
