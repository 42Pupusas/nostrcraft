use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use rand::Rng;

use crate::{
    cameras::BlockIndicator, cyberspace::encode_coordinates, nostr::POWBlockDetails,
    resources::MeshesAndMaterials, UserNostrKeys,
};
use crossbeam_channel::{unbounded, Receiver, Sender};
use cryptoxide::digest::Digest;
use cryptoxide::sha2::Sha256;

use nostro2::{
    notes::{Note, SignedNote},
    userkeys::UserKeys,
};

use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
use bevy_tokio_tasks::TokioTasksRuntime;
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinHandle;
#[cfg(not(target_arch = "wasm32"))]
use tokio_util::sync::CancellationToken;

pub fn mining_plugin(app: &mut App) {
    app.init_state::<MiningState>()
        .init_resource::<MiningChannel>()
        .init_resource::<UnminedBlockMap>()
        .init_resource::<POWNotes>()
        .add_systems(Update, (add_unmined_blocks, mining_trigger))
        .add_systems(OnEnter(MiningState::Mining), mining_system);
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum MiningState {
    #[default]
    Idle,
    Mining,
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

fn mining_trigger(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mining_channel: ResMut<MiningChannel>,
    mut state: ResMut<NextState<MiningState>>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        state.set(MiningState::Mining);
    }
    if keyboard_input.just_pressed(KeyCode::KeyN) {
        state.set(MiningState::Idle);
        let _ = mining_channel.0.send(MiningEvent);
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

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
use crate::nostr::OutgoingNotes;

#[cfg(target_arch = "wasm32")]
use bevy_wasm_tasks::WASMTasksRuntime;

#[cfg(target_arch = "wasm32")]
fn mining_system(
    mut commands: Commands,
    mut unmined_block_map: ResMut<UnminedBlockMap>,
    user_keys: Res<UserNostrKeys>,
    outgoing_notes: ResMut<OutgoingNotes>,
    runtime: ResMut<WASMTasksRuntime>,
) {
    if unmined_block_map.len() == 0 {
        return;
    }
    // This channel is used to send the mined blocks to the websocket thread
    // for broadcasting to the relay network

    let (pow_notes_writer, pow_notes_reader) = unbounded::<SignedNote>();
    commands.insert_resource(POWNotes(pow_notes_reader));

    // This channel is used to send a cancellation signal to the mining threads
    let (sender, receiver) = unbounded::<MiningEvent>();
    commands.insert_resource(MiningChannel(sender));

    // Build a list of blocks to mine
    let mut blocks = Vec::new();
    for (key, entity) in unmined_block_map.iter() {
        blocks.push(key.clone());
        // Remove the block from the scene so it doesn't get mined again
        commands.entity(*entity).despawn();
    }
    // Clear the hashmap
    unmined_block_map.clear();
    let user_keys = user_keys.get_keypair();
    runtime.spawn_background_task(|_ctx| async move {
        let writer_arc = Arc::new(pow_notes_writer);

        // We spawn a mining thread for each block
        for block in blocks {
            let writer_arc_clone = writer_arc.clone();
            let key_ref = user_keys.clone();

            let mining_thread = async move {
                mine_pow_event(block, writer_arc_clone, key_ref).await;
            };
            spawn_local(mining_thread);
        }

    });
}

#[cfg(target_arch = "wasm32")]
async fn mine_pow_event(
    coordinate: String,
    writer_arc_clone: Arc<Sender<SignedNote>>,
    key_ref: Arc<UserKeys>,
) {
    let mut pow: usize = 0;
    info!("Starting POW Miner");
    let mut block_details = POWBlockDetails {
        pow_amount: pow,
        coordinates: coordinate.clone(),
        miner_pubkey: key_ref.get_public_key(),
    };

    loop {
        let mut pow_note = Note::new(
            &key_ref.get_public_key(),
            334,
            &json!(block_details).to_string(),
        );
        let nonce = generate_nonce();
        pow_note.add_tag("nonce", &hex::encode(nonce));
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
            block_details.pow_amount = pow;
            let signed_note = key_ref.sign_nostr_event(pow_note);
            info!("Sending POW block with {} leading zeroes", pow);
            let _sent = writer_arc_clone.send(signed_note);
            info!("Sent POW block with {} leading zeroes", pow);
        }
    }
    info!("Stopping POW Miner");
}

#[cfg(not(target_arch = "wasm32"))]
fn mining_system(
    runtime: ResMut<TokioTasksRuntime>,
    mut commands: Commands,
    mut unmined_block_map: ResMut<UnminedBlockMap>,
    user_keys: Res<UserNostrKeys>,
) {
    // This channel is used to send the mined blocks to the websocket thread
    // for broadcasting to the relay network

    let (pow_notes_writer, pow_notes_reader) = unbounded::<SignedNote>();
    commands.insert_resource(POWNotes(pow_notes_reader));

    // This channel is used to send a cancellation signal to the mining threads
    let (sender, receiver) = unbounded::<MiningEvent>();
    commands.insert_resource(MiningChannel(sender));

    // Build a list of blocks to mine
    let mut blocks = Vec::new();
    for (key, entity) in unmined_block_map.iter() {
        blocks.push(key.clone());
        // Remove the block from the scene so it doesn't get mined again
        commands.entity(*entity).despawn();
    }
    // Clear the hashmap
    unmined_block_map.clear();

    let user_keys = user_keys.get_keypair();
    runtime.spawn_background_task(|_ctx| async move {
        let writer_arc = Arc::new(pow_notes_writer);
        let token = CancellationToken::new();
        let mut thread_array: Vec<JoinHandle<()>> = Vec::new();

        // We spawn a mining thread for each block
        for block in blocks {
            let writer_arc_clone = writer_arc.clone();
            let child_token = token.clone();
            let key_ref = user_keys.clone();

            let mining_thread = tokio::spawn(async move {
                mine_pow_event(block, writer_arc_clone, child_token, key_ref).await;
            });
            thread_array.push(mining_thread);
        }

        // We spawn a thread to listen for the cancellation signal
        let _ = tokio::spawn(async move {
            while let Ok(_) = receiver.recv() {
                token.cancel();
            }
        })
        .await;

        // Wait for all the mining threads to finish
        for thread in thread_array {
            thread.await.unwrap();
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
async fn mine_pow_event(
    coordinate: String,
    writer_arc_clone: Arc<Sender<SignedNote>>,
    cancel_token: CancellationToken,
    key_ref: Arc<UserKeys>,
) {
    let mut pow: usize = 0;
    info!("Starting POW Miner");
    let mut block_details = POWBlockDetails {
        pow_amount: pow,
        coordinates: coordinate.clone(),
        miner_pubkey: key_ref.get_public_key(),
    };

    while !cancel_token.is_cancelled() {
        let mut pow_note = Note::new(
            &key_ref.get_public_key(),
            3333,
            &json!(block_details).to_string(),
        );
        let nonce = generate_nonce();
        pow_note.add_tag("nonce", &hex::encode(nonce));
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
            block_details.pow_amount = pow;
            let signed_note = key_ref.sign_nostr_event(pow_note);
            let _sent = writer_arc_clone.send(signed_note);
        }
    }
    info!("Stopping POW Miner");
}


fn generate_nonce() -> [u8; 16] {
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
pub struct UnminedBlockMap(pub HashMap<String, Entity>);

impl Default for UnminedBlockMap {
    fn default() -> Self {
        UnminedBlockMap(HashMap::new())
    }
}

#[derive(Component, Deref)]
struct UnminedBlock(String);

fn add_unmined_blocks(
    mut commands: Commands,
    stuff: Res<MeshesAndMaterials>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<BlockIndicator>>,
    mut unmined_block_map: ResMut<UnminedBlockMap>,
) {
    let camera_transform = camera_query.single();
    if mouse_input.just_pressed(MouseButton::Left) {
        // Calculate the coordinates of the block and encode them
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

        // Add block at the calculated coordinates
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

        // Update the hashmap with the new block
        unmined_block_map.insert(coordinate_string, block_entity);
    }
}

// KEY 55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106
