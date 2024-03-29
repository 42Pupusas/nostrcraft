use std::sync::Arc;

use bevy::prelude::*;
use bevy_wasm_tasks::WASMTasksRuntime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use nostro2::{
    notes::SignedNote,
    relays::{NostrRelay, RelayEvents},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{mining::POWNotes, ui_camera::PowEvent};

use crate::{
    cyberspace::extract_coordinates,
    resources::{
        spawn_mined_block, spawn_pubkey_note, CoordinatesMap, MeshesAndMaterials, UniqueKeys,
    },
};

#[cfg(not(target_arch = "wasm32"))]
use bevy_tokio_tasks::TokioTasksRuntime;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
pub struct IncomingNotes(pub Receiver<SignedNote>);

#[cfg(not(target_arch = "wasm32"))]
impl Default for IncomingNotes {
    fn default() -> Self {
        let (sender, receiver) = unbounded();
        IncomingNotes(receiver)
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource)]
pub struct IncomingNotes(pub Receiver<SignedNote>, Sender<SignedNote>);
#[cfg(target_arch = "wasm32")]
impl Default for IncomingNotes {
    fn default() -> Self {
        let (sender, receiver) = unbounded();
        IncomingNotes(receiver, sender)
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct OutgoingNotes(pub Sender<SignedNote>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct POWBlockDetails {
    pub pow_amount: usize,
    pub coordinates: String,
    pub miner_pubkey: String,
}

impl POWBlockDetails {
    pub fn coordinates(&self) -> Vec3 {
        if let Ok((x, y, z)) = extract_coordinates(&self.coordinates) {
            Vec3::new(x as f32, y as f32, z as f32)
        } else {
            Vec3::new(0.0, 0.0, 0.0)
        }
    }

    pub fn display_coordinates(&self) -> String {
        let coordinates = extract_coordinates(self.coordinates.as_str()).unwrap_or((0, 0, 0));
        format!(
            "X:{}, Y: {}, Z: {}",
            coordinates.0, coordinates.1, coordinates.2
        )
    }
}

pub fn nostr_plugin(app: &mut App) {
    app.add_event::<PowEvent>()
        .init_resource::<POWNotes>()
        .init_resource::<IncomingNotes>()
        .add_systems(Startup, websocket_thread)
        .add_systems(Update, websocket_middleware);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn websocket_thread(mut commands: Commands, runtime: ResMut<TokioTasksRuntime>) {
    let (incoming_notes_sender, incoming_notes_receiver) = unbounded::<SignedNote>();
    commands.insert_resource(IncomingNotes(incoming_notes_receiver));

    let (outgoing_notes_sender, outgoing_notes_receiver) = unbounded::<SignedNote>();
    commands.insert_resource(OutgoingNotes(outgoing_notes_sender));

    runtime.spawn_background_task(|mut ctx| async move {
        if let Ok(relay) = NostrRelay::new("wss://relay.arrakis.lat").await {
            let relay_arc = Arc::new(relay);

            let relay_writer = relay_arc.clone();
            tokio::spawn(async move {
                while let Ok(note) = outgoing_notes_receiver.recv() {
                    info!("Sending note to relay {}", note);
                    let _sent = relay_writer.send_note(note).await;
                }
            });

            let relay_reader = relay_arc.clone();
            tokio::spawn(async move {
                let filter = json!({
                    "kinds": [0, 3333],
                });
                relay_reader.subscribe(filter).await.unwrap();
                while let Ok(relay_message) = relay_reader.read_relay_events().await {
                    match relay_message {
                        RelayEvents::EVENT(_, _, signed_note) => {
                            let _sent = incoming_notes_sender.send(signed_note);
                        }
                        RelayEvents::EOSE(_, _) => {
                            info!("End of Stream Event");
                        }
                        _ => {}
                    }
                }
            });
        }
    });
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[cfg(target_arch = "wasm32")]
use nostro2::{notes::Note, userkeys::UserKeys, utils::new_keys};

use gloo_timers::future::TimeoutFuture;

#[cfg(target_arch = "wasm32")]
pub fn websocket_thread(mut commands: Commands, runtime: ResMut<WASMTasksRuntime>) {
    let (outgoing_notes_sender, outgoing_notes_receiver) = unbounded::<SignedNote>();
    commands.insert_resource(OutgoingNotes(outgoing_notes_sender));

    runtime.spawn_background_task(|mut ctx| async move {
        let nostr_relay = NostrRelay::new("wss://relay.arrakis.lat").await.unwrap();
        let relay_arc = Arc::new(nostr_relay);

        let writer = relay_arc.clone();
        let writer_task = async move {
            loop {
                TimeoutFuture::new(1_000).await;
                if let Ok(note) = outgoing_notes_receiver.try_recv() {
                    info!("Sending note to relay");
                    let _sent = writer.send_note(note).await;
                } 
            }
        };
        spawn_local(writer_task);

        let reader = relay_arc.clone();

        let reader_task = async move {
            let filter = json!({
                "kinds": [0, 3333],
            });
            reader.subscribe(filter).await.unwrap();
            while let Ok(relay_message) = reader.read_relay_events().await {
                match relay_message {
                    RelayEvents::EVENT(_, _, signed_note) => {
                        ctx.run_on_main_thread(move |ctx| {
                            // The inner context gives access to a mutable Bevy World reference.
                            let world: &mut World = ctx.world;
                            let incoming_notes = world.get_resource_mut::<IncomingNotes>().unwrap();
                            incoming_notes.1.send(signed_note).unwrap();
                        })
                        .await;
                    }
                    RelayEvents::EOSE(_, _) => {
                        info!("End of Stream Event");
                    }
                    _ => {}
                }
            }
        };
        spawn_local(reader_task);
    });
}

pub fn websocket_middleware(
    mut commands: Commands,
    stuff: Res<MeshesAndMaterials>,
    incoming_notes: Res<IncomingNotes>,
    outgoing_notes: Res<OutgoingNotes>,
    pow_notes: Res<POWNotes>,
    mut pow_events: EventWriter<PowEvent>,
    mut unique_keys: ResMut<UniqueKeys>,
    mut coordinates_map: ResMut<CoordinatesMap>,
) {
    incoming_notes.0.try_iter().for_each(|note| {
        if !unique_keys.contains(note.get_pubkey()) {
            spawn_pubkey_note(&mut commands, &stuff, note.get_pubkey().to_string());
            unique_keys.insert(note.get_pubkey().to_string());
        }

        // Check if the note is a POW block with proper formatting
        if let Ok(pow_block_details) = serde_json::from_str::<POWBlockDetails>(&note.get_content())
        {
            // Check if the coordinates aalready have a block
            if !coordinates_map.contains_key(&pow_block_details.coordinates) {
                // If not, spawn a new block
                let spawned_block = spawn_mined_block(&mut commands, &stuff, &pow_block_details);
                // And add it to the hashmap
                coordinates_map.insert(
                    pow_block_details.coordinates.to_string(),
                    (spawned_block, pow_block_details.clone()),
                );
            } else {
                // Get the matching block from the hashmap
                let existing_pow_block =
                    coordinates_map.get(&pow_block_details.coordinates).unwrap();
                // Get the amount of POW for the existing block
                let existing_entity = existing_pow_block.0;

                // If the new block has more POW, replace the existing block
                if pow_block_details.pow_amount > existing_pow_block.1.pow_amount {
                    // Spawn the new block
                    let spawned_block =
                        spawn_mined_block(&mut commands, &stuff, &pow_block_details);
                    // Add it to the hashmap
                    coordinates_map.insert(
                        pow_block_details.coordinates.to_string(),
                        (spawned_block, pow_block_details.clone()),
                    );
                    // Despawn the old block
                    commands.entity(existing_entity).despawn();
                }
            }
        }
    });

    // Forward the mined POW notes to the websocket
    pow_notes.try_iter().for_each(|note| {
        info!("Forwarding POW note to websocket {}", note);
        if let Ok(block_details) = serde_json::from_str::<POWBlockDetails>(note.get_content()) {
            pow_events.send(PowEvent(block_details));
            info!("Sent POW event to websocket");
        }
        let _sent = outgoing_notes.send(note);
        info!("Sent POW note to websocket");
    });
}
