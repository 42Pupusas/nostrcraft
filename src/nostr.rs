use std::sync::Arc;

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use nostro2::{
    notes::SignedNote,
    relays::{NostrRelay, RelayEvents},
};
use serde_json::json;

use crate::{resources::{MeshesAndMaterials, UniqueKeys, spawn_pubkey_note, spawn_mined_block}, cyberspace::hex_string_to_i_space, POWEvent, POWNotes};

#[derive(Resource, Deref, DerefMut)]
pub struct IncomingNotes(pub Receiver<POWBlockDetails>);

#[derive(Resource, Deref, DerefMut)]
pub struct OutgoingNotes(pub Sender<SignedNote>);

pub struct POWBlockDetails {
    pow_amount: usize,
    coordinates: Vec3,
    miner_pubkey: String,
}

pub fn websocket_thread(mut commands: Commands, runtime: ResMut<TokioTasksRuntime>) {
    let (notes_writer, notes_reader) = unbounded::<POWBlockDetails>();
    commands.insert_resource(IncomingNotes(notes_reader));

    let (outgoing_notes_sender, outgoing_notes_receiver) = unbounded::<SignedNote>();
    commands.insert_resource(OutgoingNotes(outgoing_notes_sender));

    runtime.spawn_background_task(|_ctx| async move {
        if let Ok(relay) = NostrRelay::new("wss://relay.arrakis.lat").await {
            let filter = json!({
                "kinds": [333],
            });

            let relay_arc = Arc::new(relay);
            let relay = relay_arc.clone();

            relay.subscribe(filter).await.unwrap();

            tokio::spawn(async move {
                while let Ok(note) = outgoing_notes_receiver.recv() {
                    let sent = relay.send_note(note).await;
                    info!("Sent Note TO RELAY: {:?}", sent);
                }
            });

            let relay = relay_arc.clone();
            tokio::spawn(async move {
                while let Some(Ok(relay_message)) = relay.read_from_relay().await {
                    match relay_message {
                        RelayEvents::EVENT(_, _, signed_note) => {
                            if signed_note.get_tags_by_id("i").is_none() {
                                continue;
                            }

                            let coordinate_string = signed_note.get_content();
                            let coordinates_f32 = hex_string_to_i_space(&coordinate_string);
                            let coordinates =
                                Vec3::new(coordinates_f32.0, coordinates_f32.1, coordinates_f32.2);
                            let miner_pubkey = signed_note.get_pubkey().to_string();
                            let pow_amount = signed_note
                                .get_id()
                                .chars()
                                .take_while(|c| c == &'0')
                                .count();
                            let pow_block_details = POWBlockDetails {
                                pow_amount,
                                coordinates,
                                miner_pubkey,
                            };
                            let sent = notes_writer.send(pow_block_details);
                            info!("Sent POWBlockDetails: {:?}", sent);
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
    _pow_event: EventWriter<POWEvent>,
) {
    incoming_notes.try_iter().for_each(|note| {
        if !unique_keys.contains(&note.miner_pubkey) {
            unique_keys.insert(note.miner_pubkey.clone());
            info!("Unique Keys: {:?}", unique_keys);
            spawn_pubkey_note(&mut commands, &stuff, note.miner_pubkey.clone());
        }
        spawn_mined_block(
            &mut commands,
            &stuff,
            note.coordinates,
            note.pow_amount,
            note.miner_pubkey.clone(),
        );
    });
    pow_notes.try_iter().for_each(|note| {
        let sent = outgoing_notes.send(note);
        info!("Sent POW Note: {:?}", sent);
    });
}
