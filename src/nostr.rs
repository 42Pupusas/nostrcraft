use std::sync::Arc;

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use nostro2::{
    notes::SignedNote,
    relays::{NostrRelay, RelayEvents},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    cyberspace::{extract_coordinates, scale_coordinates_to_world},
    mining::POWNotes,
    resources::{
        spawn_mined_block, spawn_pubkey_note, CoordinatesMap, MeshesAndMaterials, UniqueKeys,
    },
    ui_camera::PowEvent,
};

#[derive(Resource, Deref, DerefMut)]
pub struct IncomingNotes(pub Receiver<SignedNote>);

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

pub fn websocket_thread(mut commands: Commands, runtime: ResMut<TokioTasksRuntime>) {
    let (notes_writer, notes_reader) = unbounded::<SignedNote>();
    commands.insert_resource(IncomingNotes(notes_reader));

    let (outgoing_notes_sender, outgoing_notes_receiver) = unbounded::<SignedNote>();
    commands.insert_resource(OutgoingNotes(outgoing_notes_sender));

    runtime.spawn_background_task(|_ctx| async move {
        if let Ok(relay) = NostrRelay::new("wss://relay.arrakis.lat").await {
            let filter = json!({
                "kinds": [0, 333],
            });

            let relay_arc = Arc::new(relay);
            let relay = relay_arc.clone();

            relay.subscribe(filter).await.unwrap();

            tokio::spawn(async move {
                while let Ok(note) = outgoing_notes_receiver.recv() {
                    let _sent = relay.send_note(note).await;
                }
            });

            let relay = relay_arc.clone();
            tokio::spawn(async move {
                while let Some(Ok(relay_message)) = relay.read_from_relay().await {
                    match relay_message {
                        RelayEvents::EVENT(_, _, signed_note) => {
                            let _ = notes_writer.send(signed_note);
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
    incoming_notes.try_iter().for_each(|note| {
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
        if let Ok(block_details) = serde_json::from_str::<POWBlockDetails>(note.get_content()) {
            pow_events.send(PowEvent(block_details));
        }
        let _sent = outgoing_notes.send(note);
    });
}
