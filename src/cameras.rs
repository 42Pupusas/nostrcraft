use crate::{
    cyberspace::{extract_coordinates, scale_coordinates_to_world},
    resources::MeshesAndMaterials,
    ui_camera::{AvatarListDetails, UiElement},
    UserNostrKeys,
};

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    input::mouse::MouseMotion,
    prelude::*,
    render::camera::RenderTarget,
    window::WindowRef,
};

pub fn camera_plugin(app: &mut App) {
    app.add_systems(PostStartup, setup_voxel_camera)
        .add_systems(
            Update,
            (
                camera_look_system,
                move_block_indicator,
                return_home,
                teleporting_to_avatar,
            ),
        );
}

const CAMERA_ORBIT_LOCATION: Vec3 = Vec3::new(4.0, 21.0, 21.0);
const CAMERA_ORBIT_LOOK_AT: Vec3 = Vec3::ZERO;

#[derive(Component)]
struct ExplorerCamera;

#[derive(Component)]
pub struct BlockIndicator {
    pub teleport_progress: f32,
}

#[derive(Bundle)]
pub struct ExplorerCameraBundle(Camera3dBundle, ExplorerCamera, BloomSettings);

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
        }
    }
}

fn setup_voxel_camera(
    mut commands: Commands,
    nostr_signer: Res<UserNostrKeys>,
    assets: Res<MeshesAndMaterials>,
) {
    commands
        .spawn((
            PbrBundle {
                mesh: assets.cube_mesh.clone_weak(),
                material: assets.clear_material.clone_weak(),
                transform: Transform::from_translation(nostr_signer.get_home_coordinates()),
                ..Default::default()
            },
            BlockIndicator {
                teleport_progress: 0.0,
            },
        ))
        .with_children(|builder| {
            builder.spawn(ExplorerCameraBundle::new_default(
                CAMERA_ORBIT_LOCATION,
                CAMERA_ORBIT_LOOK_AT,
            ));
        });
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

fn camera_look_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut camera_state: Query<&mut Transform, With<ExplorerCamera>>,
) {
    if let Ok(mut camera_transform) = camera_state.get_single_mut() {
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

fn return_home(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut block_indicator: Query<(&mut Transform, &mut BlockIndicator)>,
    nostr_signer: Res<UserNostrKeys>,
    mut text_query: Query<(&mut Text, &UiElement)>,
) {
    let (mut block_transform, mut block_details) = block_indicator.single_mut();

    if keyboard_input.pressed(KeyCode::Home) {
        while block_details.teleport_progress < 100.0 {
            block_details.teleport_progress += 1.0;
            for (mut text, ui_entity) in text_query.iter_mut() {
                if let UiElement::TeleportingNotice(_) = ui_entity {
                    text.sections[0].value =
                        format!("Going Home: {:.2}%", block_details.teleport_progress);
                }
            }
            return;
        }
        block_details.teleport_progress = 0.0;
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

        block_transform.translation = home_vec;
    }

    if keyboard_input.just_released(KeyCode::Home) {
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value = String::new();
                block_details.teleport_progress = 0.0;
            }
        }
    }
}

fn teleporting_to_avatar(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    avatar_list: ResMut<AvatarListDetails>,
    mut block_indicator: Query<(&mut BlockIndicator, &mut Transform)>,
    mut text_query: Query<(&mut Text, &UiElement)>,
) {
    let (mut block_details, mut block_transform) = block_indicator.single_mut();
    if keyboard_input.pressed(KeyCode::End) {
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value =
                    format!("Teleporting... {:.2}%", block_details.teleport_progress);
                if block_details.teleport_progress < 100.0 {
                    block_details.teleport_progress += 1.0;
                } else {
                    block_details.teleport_progress = 0.0;
                    text.sections[0].value = String::new();

                    block_transform.translation = avatar_list.get_coordinates();
                }
            }
        }
    }

    if keyboard_input.just_released(KeyCode::End) {
        for (mut text, ui_entity) in text_query.iter_mut() {
            if let UiElement::TeleportingNotice(_) = ui_entity {
                text.sections[0].value = String::new();
                block_details.teleport_progress = 0.0;
            }
        }
    }
}
