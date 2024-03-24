use crate::{
    cyberspace::{encode_coordinates, extract_coordinates, scale_coordinates_to_world},
    resources::{CoordinatesMap, MeshesAndMaterials, UniqueKeys},
    MiningKeypair,
};

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    input::mouse::MouseMotion,
    prelude::*,
    render::camera::RenderTarget,
    window::WindowRef,
};

pub fn camera_plugin(app: &mut App) {
    app.init_resource::<AvatarListDetails>()
        .add_systems(PostStartup, setup_camera_ui)
        .add_systems(
            Update,
            (
                camera_look_system,
                move_block_indicator,
                display_coordinates,
                display_avatars,
                return_home,
            ),
        );
}

#[derive(Component)]
struct ExplorerCamera;

#[derive(Component, Copy, Clone, Debug)]
struct CameraState {}

impl Default for CameraState {
    fn default() -> Self {
        CameraState {}
    }
}

const CAMERA_ORBIT_LOCATION: Vec3 = Vec3::new(4.0, 21.0, 21.0);
const CAMERA_ORBIT_LOOK_AT: Vec3 = Vec3::ZERO;

const MARGIN_UI: UiRect = UiRect::all(Val::Percent(2.1));
const PADDING_UI: UiRect = UiRect::all(Val::Percent(1.4));
const BORDER_WIDTH: UiRect = UiRect::all(Val::Px(4.2));
const LIGHT_GRAY: Color = Color::rgb(0.7, 0.7, 0.7);
const UI_FONT_SIZE: f32 = 21.0;

#[derive(Bundle)]
pub struct ExplorerCameraBundle(Camera3dBundle, ExplorerCamera, BloomSettings, CameraState);

impl ExplorerCameraBundle {
    pub fn new_default(location: Vec3, looking_at: Vec3) -> Self {
        let camera_entity = Camera3dBundle {
            camera: Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                target: RenderTarget::Window(WindowRef::Primary),
                ..Default::default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            transform: Transform::from_translation(location).looking_at(looking_at, Vec3::Y),
            ..Default::default()
        };

        ExplorerCameraBundle {
            0: camera_entity,
            1: ExplorerCamera,
            2: BloomSettings {
                intensity: 0.21,

                ..Default::default()
            },
            3: CameraState::default(),
        }
    }
}

#[derive(Component)]
pub enum UiElement {
    CurrentCoordinates,
    CoordinateString,
    CoordinateOwner,
    AvatarList(usize),
    TeleportingNotice(f32),
    MiningKey,
    MiningNotice,
}

fn setup_camera_ui(
    mut commands: Commands,
    assets: Res<MeshesAndMaterials>,
    nostr_signer: Res<MiningKeypair>,
) {
    let pubkey = nostr_signer.get_public_key();
    info!("Public Key: {:?}", pubkey);

    let home_coordinates = extract_coordinates(&pubkey).unwrap();
    let scale_coordinates =
        scale_coordinates_to_world(home_coordinates.0, home_coordinates.1, home_coordinates.2);
    let home_vec = Vec3::new(
        scale_coordinates.0,
        scale_coordinates.1,
        scale_coordinates.2,
    );
    info!(
        "Home Position: X: {} Y: {} Z: {}",
        scale_coordinates.0, scale_coordinates.1, scale_coordinates.2
    );

    commands
        .spawn((
            PbrBundle {
                mesh: assets.cube_mesh.clone_weak(),
                material: assets.clear_material.clone_weak(),
                transform: Transform::from_translation(home_vec),
                ..Default::default()
            },
            BlockIndicator,
        ))
        .with_children(|builder| {
            builder.spawn(ExplorerCameraBundle::new_default(
                CAMERA_ORBIT_LOCATION,
                CAMERA_ORBIT_LOOK_AT,
            ));
        });

    let coordinates_ui = NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            left: Val::Px(0.0),
            margin: MARGIN_UI,
            padding: PADDING_UI,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            border: BORDER_WIDTH,
            ..Default::default()
        },
        border_color: BorderColor(LIGHT_GRAY),
        ..Default::default()
    };
    commands
        .spawn(coordinates_ui)
        .with_children(|coordinates_ui| {
            let home_title = TextBundle::from_section(
                "Coordinate Hex:",
                TextStyle {
                    font_size: UI_FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                margin: MARGIN_UI,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            });
            let home_coordinates = TextBundle::from_section(
                format!("{}...{}", &pubkey[..8], &pubkey[pubkey.len() - 8..]),
                TextStyle {
                    font_size: UI_FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                margin: MARGIN_UI,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            });
            let current_coordinate_title = TextBundle::from_section(
                "Current Coordinates:",
                TextStyle {
                    font_size: UI_FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                margin: MARGIN_UI,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            });
            let current_coordinates = TextBundle::from_section(
                format!(
                    "X: {} Y: {} Z: {}",
                    scale_coordinates.0, scale_coordinates.1, scale_coordinates.2
                ),
                TextStyle {
                    font_size: UI_FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                margin: MARGIN_UI,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            });
            let coordinate_owner = TextBundle::from_section(
                "",
                TextStyle {
                    font_size: UI_FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                margin: MARGIN_UI,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            });

            coordinates_ui.spawn(current_coordinate_title);
            coordinates_ui.spawn((current_coordinates, UiElement::CurrentCoordinates));
            coordinates_ui.spawn(home_title);
            coordinates_ui.spawn((home_coordinates, UiElement::CoordinateString));
            coordinates_ui.spawn((coordinate_owner, UiElement::CoordinateOwner));
        });

    let avatars_ui = NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.0),
            right: Val::Px(0.0),
            margin: MARGIN_UI,
            padding: PADDING_UI,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            border: BORDER_WIDTH,
            ..Default::default()
        },
        border_color: BorderColor(LIGHT_GRAY),
        ..Default::default()
    };

    commands.spawn(avatars_ui).with_children(|avatars_ui| {
        let avatar_title = TextBundle::from_section(
            "Avatars:",
            TextStyle {
                font_size: UI_FONT_SIZE,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            margin: MARGIN_UI,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });
        avatars_ui.spawn(avatar_title);

        for i in 0..5 {
            let avatar_list = TextBundle::from_section(
                "",
                TextStyle {
                    font_size: UI_FONT_SIZE,
                    color: Color::WHITE,
                    ..default()
                },
            )
            .with_style(Style {
                margin: MARGIN_UI,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            });
            avatars_ui.spawn((avatar_list, UiElement::AvatarList(i)));
        }
        let teleporting_notice = TextBundle::from_section(
            "",
            TextStyle {
                font_size: UI_FONT_SIZE,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            margin: MARGIN_UI,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });
        avatars_ui.spawn((teleporting_notice, UiElement::TeleportingNotice(0.0)));
    });

    let mining_ui = NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            margin: MARGIN_UI,
            padding: PADDING_UI,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceEvenly,
            align_items: AlignItems::Center,
            border: BORDER_WIDTH,
            ..Default::default()
        },
        border_color: BorderColor(LIGHT_GRAY),
        ..Default::default()
    };
    commands.spawn(mining_ui).with_children(|mining_ui| {
        let mining_title = TextBundle::from_section(
            "Mining Details:",
            TextStyle {
                font_size: UI_FONT_SIZE,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            margin: MARGIN_UI,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });
        mining_ui.spawn(mining_title);

        let mining_key = TextBundle::from_section(
            format!(
                "Mining Key: {}...{}",
                &pubkey[..8],
                &pubkey[pubkey.len() - 8..]
            ),
            TextStyle {
                font_size: UI_FONT_SIZE,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            margin: MARGIN_UI,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });

        mining_ui.spawn((mining_key, UiElement::MiningKey));

        let mining_notice = TextBundle::from_section(
            "",
            TextStyle {
                font_size: UI_FONT_SIZE,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            margin: MARGIN_UI,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        });

        mining_ui.spawn((mining_notice, UiElement::MiningNotice));
    });
}

#[derive(Resource)]
struct AvatarListDetails {
    selected: usize,
    coordinate_string: String,
    teleport_progress: f32,
}

impl Default for AvatarListDetails {
    fn default() -> Self {
        AvatarListDetails {
            selected: 0,
            coordinate_string: String::new(),
            teleport_progress: 0.0,
        }
    }
}

fn return_home(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<BlockIndicator>>,
    nostr_signer: Res<MiningKeypair>,
    mut avatar_list: ResMut<AvatarListDetails>,
    mut text_query: Query<(&mut Text, &UiElement)>,
) {
    if keyboard_input.pressed(KeyCode::Home) {
        while avatar_list.teleport_progress < 100.0 {
            avatar_list.teleport_progress += 1.0;
            for (mut text, ui_entity) in text_query.iter_mut() {
                if let UiElement::TeleportingNotice(_) = ui_entity {
                    text.sections[0].value =
                        format!("Home... {:.2}%", avatar_list.teleport_progress);
                }
            }
            return;
        }
        avatar_list.teleport_progress = 0.0;
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value = String::new();
            }
        }
        let pubkey = nostr_signer.get_public_key();
        let home_coordinates = extract_coordinates(&pubkey).unwrap();
        let scale_coordinates =
            scale_coordinates_to_world(home_coordinates.0, home_coordinates.1, home_coordinates.2);
        let home_vec = Vec3::new(
            scale_coordinates.0,
            scale_coordinates.1,
            scale_coordinates.2,
        );

        let mut camera_transform = camera_query.single_mut();
        camera_transform.translation = home_vec;
    }

    if keyboard_input.just_released(KeyCode::Home) {
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value = String::new();
                avatar_list.teleport_progress = 0.0;
            }
        }
    }
}

fn display_avatars(
    unique_keys: Res<UniqueKeys>,
    mut text_query: Query<(&mut Text, &UiElement)>,
    mut avatar_list: ResMut<AvatarListDetails>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<BlockIndicator>>,
) {
    if unique_keys.len() == 0 {
        return;
    }

    let keys_vec: Vec<&String> = unique_keys.iter().collect(); // Convert HashSet to Vec

    let list_len = keys_vec.len();
    let middle_index = 2; // Middle index for a list of 5 items
    let selected_index = (avatar_list.selected + list_len / 2) % list_len; // Calculate selected index based on list length and ensure it's in the middle

    for (i, _key) in (0..5).enumerate() {
        let index = (selected_index + i - middle_index + list_len) % list_len; // Calculate the index to display, ensuring it wraps around and the selected index is in the middle

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

    let mut camera_transform = camera_query.single_mut();
    if keyboard_input.pressed(KeyCode::End) {
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value =
                    format!("Teleporting... {:.2}%", avatar_list.teleport_progress);
                if avatar_list.teleport_progress < 100.0 {
                    avatar_list.teleport_progress += 1.0;
                    info!("Teleporting... {:.2}%", avatar_list.teleport_progress);
                } else {
                    avatar_list.teleport_progress = 0.0;
                    text.sections[0].value = String::new();
                    info!("Selected Avatar: {}", avatar_list.coordinate_string);
                    let coordinates = extract_coordinates(&avatar_list.coordinate_string).unwrap();
                    let scale_coordinates =
                        scale_coordinates_to_world(coordinates.0, coordinates.1, coordinates.2);
                    let vec = Vec3::new(
                        scale_coordinates.0,
                        scale_coordinates.1,
                        scale_coordinates.2,
                    );

                    camera_transform.translation = vec;
                }
            }
        }
    }

    if keyboard_input.just_released(KeyCode::End) {
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value = String::new();
                avatar_list.teleport_progress = 0.0;
            }
        }
    }
}

fn camera_look_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut camera_state: Query<(&mut CameraState, &mut Transform)>,
) {
    if let Ok((_camera_state, mut camera_transform)) = camera_state.get_single_mut() {
        let vec_forward = camera_transform.rotation.mul_vec3(Vec3::Z);

        if mouse_input.pressed(MouseButton::Right) {
            let delta: Vec2 = mouse_motion_events
                .read()
                .fold(Vec2::ZERO, |acc, motion| acc + motion.delta);
            // Calculate the pitch adjustment relative to the camera's current orientation
            let right_dir = camera_transform.local_x();
            let pitch_quat = Quat::from_axis_angle(*right_dir, -delta.y * 0.01);
            camera_transform.rotate_around(Vec3::ZERO, pitch_quat);

            // Move the yaw with delta.x
            camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(delta.x * 0.01));
        }

        if mouse_input.pressed(MouseButton::Middle) {
            let delta: Vec2 = mouse_motion_events
                .read()
                .fold(Vec2::ZERO, |acc, motion| acc + motion.delta);
            camera_transform.translation += vec_forward * delta.y * 0.1;
        }
    }
}

#[derive(Component)]
pub struct BlockIndicator;

#[derive(Component)]
struct ColorText;

fn display_coordinates(
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
                        format!("X: {} Y: {} Z: {}", rounded_x, rounded_y, rounded_z);
                    text.sections[0].value = current_coordinates;
                }

                UiElement::CoordinateString => {
                    text.sections[0].value = format!(
                        "{}...{}",
                        &coordinate_string[..8],
                        &coordinate_string[coordinate_string.len() - 8..]
                    );
                }
                UiElement::CoordinateOwner => {
                    if let Some(owner) = mined_blocks.get(&coordinate_string) {
                        text.sections[0].value = format!(
                            "Owner: {}...{}",
                            &owner.1.miner_pubkey[..8],
                            &owner.1.miner_pubkey[owner.1.miner_pubkey.len() - 8..]
                        );
                    } else {
                        text.sections[0].value = String::new();
                    }
                }
                _ => {}
            }
        }
    }
}

fn move_block_indicator(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &BlockIndicator)>,
) {
    for (mut transform, _block_indicator) in query.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::KeyW) {
            transform.translation.z -= 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyS) {
            transform.translation.z += 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyA) {
            transform.translation.x -= 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyD) {
            transform.translation.x += 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyQ) {
            transform.translation.y += 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyE) {
            transform.translation.y -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowUp) {
            transform.translation.z -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowDown) {
            transform.translation.z += 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowRight) {
            transform.translation.x += 1.0;
        }

        if keyboard_input.pressed(KeyCode::PageUp) {
            transform.translation.y += 1.0;
        }

        if keyboard_input.pressed(KeyCode::PageDown) {
            transform.translation.y -= 1.0;
        }
    }
}
