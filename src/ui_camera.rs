use bevy::prelude::*;

use crate::{
    cameras::BlockIndicator,
    cyberspace::{encode_coordinates, extract_coordinates, scale_coordinates_to_world},
    mining::{MiningState, UnminedBlockMap},
    nostr::POWBlockDetails,
    resources::{CoordinatesMap, UniqueKeys},
    UserNostrKeys,
};

pub fn ui_camera_plugin(app: &mut App) {
    app.init_resource::<AvatarListDetails>()
        .add_systems(
            PostStartup,
            (setup_coordinate_ui, setup_avatar_list, setup_mining_ui),
        )
        .add_systems(
            Update,
            (update_coordinate_ui, update_avatar_list),
        );
}

#[derive(Component)]
pub enum UiElement {
    CurrentCoordinates,
    AvatarList(usize),
    TeleportingNotice(f32),
    MiningKey,
    MiningNotice,
}

const FLEX_GAP: Val = Val::Px(8.4);
const MARGIN_UI: UiRect = UiRect::all(Val::Percent(2.1));
const PADDING_UI: UiRect = UiRect::all(Val::Percent(0.7));
const BORDER_WIDTH: UiRect = UiRect::all(Val::Px(4.2));
const LIGHT_GRAY: Color = Color::rgb(0.7, 0.7, 0.7);
const TITLE_FONT: f32 = 18.0;
const NORMAL_FONT: f32 = 12.0;

fn setup_coordinate_ui(mut commands: Commands) {
    let coordinates_ui = NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            margin: MARGIN_UI,
            padding: PADDING_UI,
            row_gap: FLEX_GAP,
            column_gap: FLEX_GAP,
            flex_direction: FlexDirection::Column,
            border: BORDER_WIDTH,
            ..Default::default()
        },
        border_color: BorderColor(LIGHT_GRAY),
        ..Default::default()
    };

    commands
        .spawn(coordinates_ui)
        .with_children(|coordinates_ui| {
            let current_coordinate_title =
                text_bundle_builder("Current Coordinates".to_string(), TITLE_FONT);
            coordinates_ui.spawn(current_coordinate_title);

            let current_coordinates = multi_section_text_builder(3);
            coordinates_ui.spawn((current_coordinates, UiElement::CurrentCoordinates));
        });
}

fn setup_avatar_list(mut commands: Commands) {
    let avatars_ui = NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            right: Val::Px(0.0),
            margin: MARGIN_UI,
            padding: PADDING_UI,
            row_gap: FLEX_GAP,
            column_gap: FLEX_GAP,
            flex_direction: FlexDirection::Column,
            border: BORDER_WIDTH,
            ..Default::default()
        },
        border_color: BorderColor(LIGHT_GRAY),
        ..Default::default()
    };

    commands.spawn(avatars_ui).with_children(|avatars_ui| {
        let avatar_title = text_bundle_builder("Avatars".to_string(), TITLE_FONT);
        avatars_ui.spawn(avatar_title);

        for i in 0..5 {
            let avatar_list = text_bundle_builder(String::new(), NORMAL_FONT);
            avatars_ui.spawn((avatar_list, UiElement::AvatarList(i)));
        }
        let teleporting_notice = text_bundle_builder(String::new(), TITLE_FONT);
        avatars_ui.spawn((teleporting_notice, UiElement::TeleportingNotice(0.0)));
    });
}

#[derive(Resource)]
pub struct AvatarListDetails {
    selected: usize,
    coordinate_string: String,
}

impl AvatarListDetails {
    pub fn get_coordinates(&self) -> Vec3 {
        let i128_coordinates = extract_coordinates(&self.coordinate_string).unwrap_or((0, 0, 0));
        let world_coordinates =
            scale_coordinates_to_world(i128_coordinates.0, i128_coordinates.1, i128_coordinates.2);
        Vec3::new(
            world_coordinates.0 as f32,
            world_coordinates.1 as f32,
            world_coordinates.2 as f32,
        )
    }
}

impl Default for AvatarListDetails {
    fn default() -> Self {
        AvatarListDetails {
            selected: 0,
            coordinate_string: String::new(),
        }
    }
}

fn update_avatar_list(
    unique_keys: Res<UniqueKeys>,
    mut text_query: Query<(&mut Text, &UiElement)>,
    mut avatar_list: ResMut<AvatarListDetails>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if unique_keys.len() < 5 {
        return;
    }

    let keys_vec: Vec<&String> = unique_keys.iter().collect(); // Convert HashSet to Vec
                                                               
     if keys_vec.is_empty() {
        return;
    }

    let list_len = keys_vec.len();
    let middle_index = 2; // Middle index for a list of 5 items
    let selected_index = (avatar_list.selected + list_len / 2) % list_len; // Calculate selected index based on list length and ensure it's in the middle

     if list_len == 0 {
        return; // Return early if the list length is zero
    }

    for (i, _key) in (0..5).enumerate() {
        let index = (selected_index + i + list_len - middle_index) % list_len;

        // Get the corresponding Text component and UiElement tag
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::AvatarList(j) = ui_entity {
                if j == &i {
                    let avatar_key = keys_vec[index];
                    text.sections[0].value = format!(
                        "{}...{}",
                        &avatar_key[..8],
                        &avatar_key[avatar_key.len() - 8..]
                    );
                    // Set text color based on whether the current index matches the selected index
                    if index == selected_index {
                        text.sections[0].style.color = Color::GREEN;
                        avatar_list.coordinate_string = avatar_key.to_string();
                    } else {
                        text.sections[0].style.color = Color::WHITE;
                    }
                }
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::Delete) {
        avatar_list.selected = (avatar_list.selected + 1) % list_len; // Wrap around when reaching the end
    }

    if keyboard_input.just_pressed(KeyCode::Insert) {
        avatar_list.selected = (avatar_list.selected + list_len - 1) % list_len;
        // Wrap around when reaching the beginning
    }
}

fn update_coordinate_ui(
    query: Query<&Transform, With<BlockIndicator>>,
    mut text_query: Query<(&mut Text, &UiElement)>,
    mined_blocks: Res<CoordinatesMap>,
) {
    if let Ok(transform) = query.get_single() {
        let x = transform.translation.x;
        let y = transform.translation.y;
        let z = transform.translation.z;

        let rounded_x = x.round();
        let rounded_y = y.round();
        let rounded_z = z.round();

        let x_i128 = rounded_x as i128;
        let y_i128 = rounded_y as i128;
        let z_i128 = rounded_z as i128;

        let coordinate_string = encode_coordinates(x_i128, y_i128, z_i128);

        for (mut text, ui_entity) in text_query.iter_mut() {
            match ui_entity {
                UiElement::CurrentCoordinates => {
                    let current_coordinates =
                        format!("X: {} Y: {} Z: {}\n", rounded_x, rounded_y, rounded_z);
                    text.sections[0].value = current_coordinates;
                    text.sections[1].value = format!(
                        "i-Space: {}...{}\n",
                        &coordinate_string[..8],
                        &coordinate_string[coordinate_string.len() - 8..]
                    );
                    if let Some(owner) = mined_blocks.get(&coordinate_string) {
                        text.sections[2].value = format!(
                            "Owner: {}...{}",
                            &owner.1.miner_pubkey[..8],
                            &owner.1.miner_pubkey[owner.1.miner_pubkey.len() - 8..]
                        );
                    } else {
                        text.sections[2].value = String::new();
                    }
                }

                _ => {}
            }
        }
    }
}

fn setup_mining_ui(mut commands: Commands, nostr_signer: Res<UserNostrKeys>) {
    let mining_ui = NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            margin: MARGIN_UI,
            padding: PADDING_UI,
            row_gap: FLEX_GAP,
            column_gap: FLEX_GAP,
            flex_direction: FlexDirection::Column,
            border: BORDER_WIDTH,
            ..Default::default()
        },
        border_color: BorderColor(LIGHT_GRAY),
        ..Default::default()
    };

    commands.spawn(mining_ui).with_children(|mining_ui| {
        let mining_title = text_bundle_builder("Mining Details".to_string(), TITLE_FONT);
        let mining_key = text_bundle_builder(nostr_signer.get_display_key(), NORMAL_FONT);
        mining_ui.spawn(mining_title);
        mining_ui.spawn((mining_key, UiElement::MiningKey));

        let mining_notices = multi_section_text_builder(3);
        mining_ui.spawn((mining_notices, UiElement::MiningNotice));
    });
}

#[derive(Event)]
pub struct PowEvent(pub POWBlockDetails);

#[cfg(not(target_arch = "wasm32"))]
fn update_mining_ui(
    mut text_query: Query<(&mut Text, &UiElement)>,
    mining_state: Res<State<MiningState>>,
    mined_blocks: Res<CoordinatesMap>,
    unmined_blocks: Res<UnminedBlockMap>,
    mut pow_events: EventReader<PowEvent>,
) {
    let blocks_in_world = mined_blocks.len();
    let blocks_in_memory = unmined_blocks.len();
    for (mut text, ui_entity) in text_query.iter_mut() {
        match ui_entity {
            UiElement::MiningNotice => match mining_state.get() {
                MiningState::Idle => {
                    text.sections[0].value = format!("Blocks in world: {}\n", blocks_in_world);
                    text.sections[1].value = format!("Unmined Blocks: {}\n", blocks_in_memory);
                    text.sections[2].value = if blocks_in_memory > 0 {
                        "Press M to mine".to_string()
                    } else {
                        "No blocks to mine".to_string()
                    };
                }
                MiningState::Mining => {
                    text.sections[0].value = "Mining... Press N to stop\n".to_string();
                    for event in pow_events.read() {
                        let block = &event.0;
                        text.sections[1].value =
                            format!("Mined block at: {}\n", block.display_coordinates());
                        text.sections[2].value = format!("With POW: {}\n", block.pow_amount);
                    }
                }
            },

            _ => {}
        }
    }
}

fn text_bundle_builder(content: String, font_size: f32) -> TextBundle {
    TextBundle::from_section(
        content,
        TextStyle {
            font_size,
            color: Color::WHITE,
            ..default()
        },
    )
    .with_style(Style {
        margin: MARGIN_UI,
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        ..Default::default()
    })
}

fn multi_section_text_builder(sections: usize) -> TextBundle {
    let mut text_sections = Vec::new();
    for _ in 0..sections {
        text_sections.push(TextSection {
            value: String::new(),
            style: TextStyle {
                font_size: NORMAL_FONT,
                color: Color::WHITE,
                ..default()
            },
        });
    }

    TextBundle::from_sections(text_sections)
}
