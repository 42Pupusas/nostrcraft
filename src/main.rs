use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use bevy_tokio_tasks::{TokioTasksPlugin, TokioTasksRuntime};

use cameras::{camera_plugin, BlockIndicator, UiElement};
use cryptoxide::digest::Digest;
use cryptoxide::sha2::Sha256;
use nostr::{websocket_middleware, websocket_thread};
use nostro2::{
    notes::{Note, SignedNote},
    userkeys::UserKeys,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use resources::{world_plugin, MeshesAndMaterials};
use serde_json::json;
use tokio::task::JoinHandle;

use crate::{cyberspace::encode_coordinates, nostr::POWBlockDetails};

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
            // FrameTimeDiagnosticsPlugin,
            // Adds a system that prints diagnostics to the console
            // LogDiagnosticsPlugin::default(),
            // Any plugin can register diagnostics. Uncomment this to add an entity count diagnostics:
            // bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            // Uncomment this to add an asset count diagnostics:
            // bevy::asset::diagnostic::AssetCountDiagnosticsPlugin::<Texture>::default(),
            // Uncomment this to add system info diagnostics:
            // bevy::diagnostic::SystemInformationDiagnosticsPlugin::default(),
        ))
        .init_state::<MiningState>()
        // Events work as a way to pass data between systems
        .add_event::<POWEvent>()
        .init_resource::<POWNotes>()
        .init_resource::<MiningChannel>()
        .init_resource::<UnminedBlockMap>()
        .init_resource::<MiningKeypair>()
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
            coordinates: String::new(),
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
    mining_channel: ResMut<MiningChannel>,
    unmined_block_map: Res<UnminedBlockMap>,
    mut mining_text: Query<(&mut Text, &UiElement)>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        state.set(MiningState::Mining);
        info!("Mining State: Mining");
        for (mut text, ui_element) in mining_text.iter_mut() {
            match ui_element {
                UiElement::MiningNotice => {
                    text.sections[0].value = format!("Mining: {} blocks", unmined_block_map.len());
                }
                _ => {}
            }
        }
    }
    if keyboard_input.just_pressed(KeyCode::KeyN) {
        state.set(MiningState::Idle);
        info!("Mining State: Idle");
        let _ = mining_channel.0.send(MiningEvent);
        for (mut text, ui_element) in mining_text.iter_mut() {
            match ui_element {
                UiElement::MiningNotice => {
                    text.sections[0].value = String::new();
                }
                _ => {}
            }
        }
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

use tokio_util::sync::CancellationToken;

#[derive(Resource, Deref, DerefMut)]
struct MiningKeypair(pub Arc<UserKeys>);

impl Default for MiningKeypair {
    fn default() -> Self {
        MiningKeypair(Arc::new(
            UserKeys::new("55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106")
                .unwrap(),
        ))
    }
}

fn mining_system(
    runtime: ResMut<TokioTasksRuntime>,
    mut commands: Commands,
    mut unmined_block_map: ResMut<UnminedBlockMap>,
    user_keys: Res<MiningKeypair>,
) {
    let (pow_notes_writer, pow_notes_reader) = unbounded::<SignedNote>();
    commands.insert_resource(POWNotes(pow_notes_reader));

    let mut blocks = Vec::new();
    for (key, entity) in unmined_block_map.iter() {
        blocks.push(key.clone());
        commands.entity(*entity).despawn();
    }

    unmined_block_map.clear();

    let user_keys = user_keys.clone();
    let (sender, receiver) = unbounded::<MiningEvent>();
    commands.insert_resource(MiningChannel(sender));

    runtime.spawn_background_task(|_ctx| async move {
        let writer_arc = Arc::new(pow_notes_writer);
        let token = CancellationToken::new();
        let mut thread_array: Vec<JoinHandle<()>> = Vec::new();

        for block in blocks {
            let writer_arc_clone = writer_arc.clone();

            let child_token = token.clone();
            let key_ref = user_keys.clone();

            let mining_thread = tokio::spawn(async move {
                mine_pow_event(block, writer_arc_clone, child_token, key_ref).await;
            });
            thread_array.push(mining_thread);
        }

        let _ = tokio::spawn(async move {
            while let Ok(_) = receiver.recv() {
                token.cancel();
            }
        })
        .await;

        for thread in thread_array {
            thread.await.unwrap();
        }
    });
}

async fn mine_pow_event(
    coordinate: String,
    writer_arc_clone: Arc<Sender<SignedNote>>,
    cancel_token: CancellationToken,
    key_ref: Arc<UserKeys>,
) {
    let mut pow: usize = 0;
    info!("Starting POW Miner");

    while !cancel_token.is_cancelled() {
        let block_details = POWBlockDetails {
            pow_amount: pow,
            coordinates: coordinate.clone(),
            miner_pubkey: key_ref.get_public_key(),
        };
        let mut pow_note = Note::new(
            key_ref.get_public_key(),
            333,
            &json!(block_details).to_string(),
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
            let signed_note = key_ref.sign_nostr_event(pow_note);
            let _sent = writer_arc_clone.send(signed_note);
            info!("New POW Amount: {}", pow);
        }
    }
    info!("Stopping POW Miner");
}

struct MiningEvent;

#[derive(Resource, Debug)]
struct MiningChannel(pub Sender<MiningEvent>);

impl Default for MiningChannel {
    fn default() -> Self {
        let (sender, _) = unbounded();
        MiningChannel(sender)
    }
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

#[derive(Resource, Debug, Deref, DerefMut)]
struct UnminedBlockMap(pub HashMap<String, Entity>);

impl Default for UnminedBlockMap {
    fn default() -> Self {
        UnminedBlockMap(HashMap::new())
    }
}

fn add_blocks(
    mut commands: Commands,
    stuff: Res<MeshesAndMaterials>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<BlockIndicator>>,
    mut unmined_block_map: ResMut<UnminedBlockMap>,
) {
    let camera_transform = camera_query.single();
    if mouse_input.just_pressed(MouseButton::Left) {
        // Assuming `camera_transform` is of type `Transform` containing translation and rotation data

        let x = camera_transform.translation.x;
        let y = camera_transform.translation.y;
        let z = camera_transform.translation.z;

        let rounded_x = x.round();
        let rounded_y = y.round();
        let rounded_z = z.round();

        let x_128 = rounded_x as i128;
        let y_128 = rounded_y as i128;
        let z_128 = rounded_z as i128;

        let coordinate_string = encode_coordinates(x_128, y_128, z_128);

        // Check if the block already exists
        if let Some(entity) = unmined_block_map.get(&coordinate_string) {
            // Remove the block
            commands.entity(*entity).despawn();
            unmined_block_map.0.remove(&coordinate_string);
            return;
        }

        let block_entity = commands
            .spawn((
                PbrBundle {
                    mesh: stuff.cube_mesh.clone_weak(),
                    material: stuff.mud_material.clone_weak(),
                    transform: Transform::from_translation(Vec3::new(
                        rounded_x, rounded_y, rounded_z,
                    ))
                    .with_rotation(Quat::IDENTITY),
                    ..Default::default()
                },
                UnminedBlock(coordinate_string.clone()),
            ))
            .id();
        unmined_block_map.insert(coordinate_string, block_entity);

        // Add block at the calculated coordinates
    }
}

#[derive(Component, Deref)]
struct UnminedBlock(String);

// KEY 55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106
