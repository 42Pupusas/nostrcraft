use crate::{
    cyberspace::{extract_coordinates, scale_coordinates_to_world},
    resources::{MeshesAndMaterials, NostrSigner},
};

use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    input::mouse::MouseMotion,
    prelude::*,
    render::camera::RenderTarget,
    window::WindowRef,
};

pub fn camera_plugin(app: &mut App) {
    app.add_systems(PostStartup, setup_camera_ui).add_systems(
        Update,
        (
            camera_look_system,
            move_block_indicator,
            display_coordinates,
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

fn setup_camera_ui(
    mut commands: Commands,
    assets: Res<MeshesAndMaterials>,
    nostr_signer: Res<NostrSigner>,
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
    info!("Scaled Coordinates: {:?}", scale_coordinates);

    let location = Vec3::new(0., 0., -10.0);
    let look_at = Vec3::ZERO;
    // Setup a ghot block to indicate the player's location
    // And a camera as a child so it will orbit the block
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
            builder.spawn(ExplorerCameraBundle::new_default(location, look_at));
        });

    // Creates small UI node on the corner to sow coordinates
    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "hello\nbevy!",
            TextStyle {
                font_size: 21.0,
                ..default()
            },
        ) // Set the justification of the Text
        .with_text_justify(JustifyText::Center)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        }),
        ColorText,
    ));
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
    mut text_query: Query<&mut Text>,
) {
    // need to log the coordinates of the mouse click
    let transform = query.single();
    let x = transform.translation.x;
    let y = transform.translation.y;
    let z = transform.translation.z;
    for mut text in text_query.iter_mut() {
        text.sections[0].value = format!("x: {}\ny: {}\nz: {}", x, y, z);
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
