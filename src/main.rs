use std::sync::Arc;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};

use cameras::{camera_plugin};
use cyberspace::{
    generate_nonce,
};
use nostr::{websocket_middleware, websocket_thread};
use nostro2::{
    notes::{Note, SignedNote},
    userkeys::UserKeys,
};

use crossbeam_channel::{unbounded, Receiver};
use resources::{
    world_plugin, BlockIndicator, MeshesAndMaterials,
};



mod cameras;
mod cyberspace;
mod resources;

mod nostr;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Tijaxx".into(),
                    prevent_default_event_handling: true,
                    focused: true,
                    resizable: true,
                    decorations: false,
                    transparent: true,
                    ..default()
                }),
                ..default()
            }),
            // Adds frame time diagnostics
            FrameTimeDiagnosticsPlugin,
            // Adds a system that prints diagnostics to the console
            LogDiagnosticsPlugin::default(),
            // Any plugin can register diagnostics. Uncomment this to add an entity count diagnostics:
            bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            // Uncomment this to add an asset count diagnostics:
            // bevy::asset::diagnostic::AssetCountDiagnosticsPlugin::<Texture>::default(),
            // Uncomment this to add system info diagnostics:
            bevy::diagnostic::SystemInformationDiagnosticsPlugin::default(),
        ))
        .init_state::<MiningState>()
        // Events work as a way to pass data between systems
        .add_event::<POWEvent>()
        .add_systems(Startup, websocket_thread)
        .add_systems(Update, (add_blocks, mining_trigger, websocket_middleware))
        .add_systems(OnEnter(MiningState::Mining), mining_system)
        .add_plugins((camera_plugin, world_plugin))
        .add_plugins(TokioTasksPlugin::default())
        .run();
}

#[derive(Event)]
struct POWEvent;

impl Default for POWEvent {
    fn default() -> Self {
        POWEvent
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum MiningState {
    #[default]
    Idle,
    Mining,
}

fn mining_trigger(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<NextState<MiningState>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        state.set(MiningState::Mining);
    }
    if keyboard_input.just_pressed(KeyCode::KeyN) {
        state.set(MiningState::Idle);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct POWNotes(pub Receiver<SignedNote>);

// Buggy as hell, still need to figure out how to turn off the mining thread
fn mining_system(
    block_query: Query<&CoordinateBlock>,
    runtime: ResMut<TokioTasksRuntime>,
    mut commands: Commands,
) {
    let (pow_notes_writer, pow_notes_reader) = unbounded::<SignedNote>();
    commands.insert_resource(POWNotes(pow_notes_reader));
    let mut blocks = Vec::new();
    for block in block_query.iter() {
        let x = block.0;
        let y = block.1;
        let z = block.2;
        blocks.push((x, y, z));
    }
    runtime.spawn_background_task(|_ctx| async move {
        let writer_arc = Arc::new(pow_notes_writer);

        for coordinates in blocks {
            let writer_arc_clone = writer_arc.clone();
            tokio::spawn(async move {
                let mut pow: usize = 0;
                let user_keys = UserKeys::new(
                    "55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106",
                )
                .unwrap();
                info!("Starting POW Miner");
                let coordinate_string =
                    i_space_to_hex_string(coordinates.0, coordinates.1, coordinates.2);

                while pow < 8 {
                    let mut pow_note =
                        Note::new(user_keys.get_public_key(), 333, &coordinate_string);
                    let nonce = generate_nonce();
                    pow_note.tag_note("nonce", &hex::encode(nonce));
                    pow_note.tag_note("nonce", &coordinate_string);
                    pow_note.tag_note("i", &coordinate_string);
                    let json_str = pow_note.serialize_for_nostr();

                    // Compute the SHA256 hash of the serialized JSON string
                    let mut hasher = Sha256::new();
                    hasher.input_str(&json_str);
                    let mut result = [0u8; 32];
                    hasher.result(&mut result);

                    let pow_id = hex::encode(result);

                    let leading_zeroes_in_id = pow_id.chars().take_while(|c| c == &'0').count();
                    if leading_zeroes_in_id > pow {
                        pow = leading_zeroes_in_id;
                        let signed_note = user_keys.sign_nostr_event(pow_note);
                        let sent = writer_arc_clone.send(signed_note);
                        info!("Sent POW Note: {:?}", sent);
                    }
                }
            });
        }
    });
}

fn add_blocks(
    mut commands: Commands,
    stuff: Res<MeshesAndMaterials>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<BlockIndicator>>,
) {
    let camera_transform = camera_query.single();
    if mouse_input.just_pressed(MouseButton::Left) {
        // Assuming `camera_transform` is of type `Transform` containing translation and rotation data

        let x = camera_transform.translation.x;
        let y = camera_transform.translation.y;
        let z = camera_transform.translation.z;

        // Add block at the calculated coordinates
        add_block_at_coordinates(&mut commands, stuff, x, y, z);
    }
}

#[derive(Component)]
struct CoordinateBlock(pub f32, pub f32, pub f32);

fn add_block_at_coordinates(
    commands: &mut Commands,
    stuff: Res<MeshesAndMaterials>,
    x: f32,
    y: f32,
    z: f32,
) {
    let rounded_x = x.round();
    let rounded_y = y.round();
    let rounded_z = z.round();

    commands.spawn((
        PbrBundle {
            mesh: stuff.cube_mesh.clone_weak(),
            material: stuff.mud_material.clone_weak(),
            transform: Transform::from_translation(Vec3::new(rounded_x, rounded_y, rounded_z))
                .with_rotation(Quat::IDENTITY),
            ..Default::default()
        },
        CoordinateBlock(rounded_x, rounded_y, rounded_z),
    ));
}

use crate::cyberspace::{i_space_to_hex_string};
use cryptoxide::digest::Digest;
use cryptoxide::sha2::Sha256;

fn pow_block_manager() {
    // TODO
    // This system should be responsible for managing the POW blocks
    // It should remove lower POW from coordinates_map if they clash
}

// KEY 55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106
