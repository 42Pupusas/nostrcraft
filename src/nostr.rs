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
    resources::{
        spawn_mined_block, spawn_pubkey_note, CoordinatesMap, MeshesAndMaterials, UniqueKeys,
    },
    POWEvent, POWNotes,
};

#[derive(Resource, Deref, DerefMut)]
pub struct IncomingNotes(pub Receiver<SignedNote>);

#[derive(Resource, Deref, DerefMut)]
pub struct OutgoingNotes(pub Sender<SignedNote>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct POWBlockDetails {
    pub pow_amount: usize,
    pub coordinates: Vec3,
    pub miner_pubkey: String,
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
                    let sent = relay.send_note(note).await;
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
    incoming_notes: Res<IncomingNotes>,
    outgoing_notes: Res<OutgoingNotes>,
    pow_notes: Res<POWNotes>,
    mut commands: Commands,
    stuff: Res<MeshesAndMaterials>,
    mut unique_keys: ResMut<UniqueKeys>,
    mut coordinates_map: ResMut<CoordinatesMap>,
    mut pow_event: EventWriter<POWEvent>,
) {
    incoming_notes.try_iter().for_each(|note| {
        if !unique_keys.contains(note.get_pubkey()) {
            spawn_pubkey_note(&mut commands, &stuff, note.get_pubkey().to_string());
            unique_keys.insert(note.get_pubkey().to_string());
        }

        if let Ok(pow_block_details) = serde_json::from_str::<POWBlockDetails>(&note.get_content())
        {
            if !coordinates_map.contains_key(&pow_block_details.coordinates.to_string()) {
                let spawned_block = spawn_mined_block(
                    &mut commands,
                    &stuff,
                    pow_block_details.coordinates,
                    pow_block_details.pow_amount,
                    pow_block_details.miner_pubkey.clone(),
                );
                coordinates_map.insert(
                    pow_block_details.coordinates.to_string(),
                    (spawned_block, pow_block_details.clone()),
                );
            } else {
                let existing_pow_block = coordinates_map
                    .get(&pow_block_details.coordinates.to_string())
                    .unwrap();

                let existing_entity = existing_pow_block.0;

                if pow_block_details.pow_amount > existing_pow_block.1.pow_amount {
                    let spawned_block = spawn_mined_block(
                        &mut commands,
                        &stuff,
                        pow_block_details.coordinates,
                        pow_block_details.pow_amount,
                        pow_block_details.miner_pubkey.clone(),
                    );
                    coordinates_map.insert(
                        pow_block_details.coordinates.to_string(),
                        (spawned_block, pow_block_details.clone()),
                    );
                    commands.entity(existing_entity).despawn();
                }
            }
        }
    });

    pow_notes.try_iter().for_each(|note| {
        let sent = outgoing_notes.send(note);
    });
}
