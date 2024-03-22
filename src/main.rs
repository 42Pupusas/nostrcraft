use std::sync::Arc;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};

use cameras::{camera_plugin, BlockIndicator};
use cryptoxide::digest::Digest;
use cryptoxide::sha2::Sha256;
use nostr::{websocket_middleware, websocket_thread};
use nostro2::{
    notes::{Note, SignedNote},
    userkeys::UserKeys,
};

use crossbeam_channel::{unbounded, Receiver};
use resources::{world_plugin, CoordinatesMap, MeshesAndMaterials, POWBlock};
use serde_json::json;

use crate::nostr::POWBlockDetails;

mod cameras;
mod cyberspace;
mod resources;

mod nostr;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "NostrCraft".into(),
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
            // bevy::diagnostic::SystemInformationDiagnosticsPlugin::default(),
        ))
        .init_state::<MiningState>()
        // Events work as a way to pass data between systems
        .add_event::<POWEvent>()
        .init_resource::<POWNotes>()
        .add_systems(Startup, websocket_thread)
        .add_systems(Update, (add_blocks, mining_trigger, websocket_middleware))
        .add_systems(OnEnter(MiningState::Mining), mining_system)
        .add_plugins((camera_plugin, world_plugin))
        .add_plugins(TokioTasksPlugin::default())
        .run();
}

#[derive(Event)]
struct POWEvent(POWBlockDetails);

impl Default for POWEvent {
    fn default() -> Self {
        POWEvent(POWBlockDetails {
            pow_amount: 0,
            coordinates: Vec3::new(0.0, 0.0, 0.0),
            miner_pubkey: String::new(),
        })
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

impl Default for POWNotes {
    fn default() -> Self {
        let (_notes_writer, notes_reader) = unbounded::<SignedNote>();
        POWNotes(notes_reader)
    }
}

// Buggy as hell, still need to figure out how to turn off the mining thread
fn mining_system(
    block_query: Query<&UnminedBlock>,
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
                    format!("{},{},{}", coordinates.0, coordinates.1, coordinates.2);

                while pow < 8 {
                    let pow_block = POWBlockDetails {
                        pow_amount: pow,
                        coordinates: Vec3::new(coordinates.0, coordinates.1, coordinates.2),
                        miner_pubkey: user_keys.get_public_key(),
                    };

                    let mut pow_note = Note::new(
                        user_keys.get_public_key(),
                        333,
                        &json!(pow_block).to_string(),
                    );
                    let nonce = generate_nonce();
                    pow_note.tag_note("nonce", &hex::encode(nonce));
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

use rand::Rng;

pub fn generate_nonce() -> [u8; 16] {
    // Define the symbols allowed in the nonce
    let symbols: [u8; 16] = [
        b'!', b'"', b'#', b'$', b'%', b'&', b'\'', b'(', b')', b'*', b'+', b',', b'-', b'.', b'/',
        b'0',
    ];

    let mut rng = rand::thread_rng();
    let mut nonce: [u8; 16] = [0; 16];

    for i in 0..16 {
        // Generate a random index to select a symbol from the array
        let index = rng.gen_range(0..16);
        // Assign the selected symbol to the nonce buffer
        nonce[i] = symbols[index];
    }

    nonce
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
struct UnminedBlock(pub f32, pub f32, pub f32);

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
        UnminedBlock(rounded_x, rounded_y, rounded_z),
    ));
}

// KEY 55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106
