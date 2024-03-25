use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksPlugin;

mod cyberspace;

mod cameras;
use cameras::camera_plugin;

mod ui_camera;

mod mining;
use cyberspace::{extract_coordinates, scale_coordinates_to_world};
use mining::mining_plugin;

mod resources;
use nostro2::userkeys::UserKeys;
use resources::world_plugin;

mod nostr;
use nostr::{websocket_middleware, websocket_thread};

use openssl::ec::EcKey;
use std::sync::Arc;
use ui_camera::ui_camera_plugin;

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
        .init_resource::<UserNostrKeys>()
        .add_systems(Startup, websocket_thread)
        .add_systems(PostStartup, add_sample_blocks)
        .add_systems(Update, websocket_middleware)
        .add_plugins((camera_plugin, world_plugin, mining_plugin, ui_camera_plugin))
        .add_plugins(TokioTasksPlugin::default())
        .run();
}

const PEM_FILE_PATH: &str = "./nostr.pem";
const DEFULT_KEYPAIR: &str = "55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106";

#[derive(Resource)]
struct UserNostrKeys {
    keypair: Arc<UserKeys>,
    home_coordinates: Vec3,
    public_key: String,
}

impl UserNostrKeys {
    fn get_keypair(&self) -> Arc<UserKeys> {
        self.keypair.clone()
    }

    fn get_home_coordinates(&self) -> Vec3 {
        self.home_coordinates
    }

    fn get_public_key(&self) -> String {
        self.public_key.clone()
    }
    fn get_display_key(&self) -> String {
        format!(
            "Your Key: {}...{}",
            &self.public_key[..8],
            &self.public_key[self.public_key.len() - 8..]
        )
    }
}

impl Default for UserNostrKeys {
    fn default() -> Self {
        let default_keypair = Arc::new(UserKeys::new(DEFULT_KEYPAIR).unwrap());
        let default_pubkey = default_keypair.get_public_key();
        let default_home_coordinates = extract_coordinates(&default_pubkey).unwrap_or((0, 0, 0));
        let scaled_home_coordinates = scale_coordinates_to_world(
            default_home_coordinates.0,
            default_home_coordinates.1,
            default_home_coordinates.2,
        );
        let home_vec3 = Vec3::new(
            scaled_home_coordinates.0 as f32,
            scaled_home_coordinates.1 as f32,
            scaled_home_coordinates.2 as f32,
        );
        let default_keys = UserNostrKeys {
            keypair: default_keypair,
            home_coordinates: home_vec3,
            public_key: default_pubkey,
        };

        let pem_file = std::fs::read(PEM_FILE_PATH);
        if pem_file.is_err() {
            return default_keys;
        }
        let pem_file = pem_file.unwrap();

        let buffer = EcKey::private_key_from_pem(&pem_file);
        if buffer.is_err() {
            return default_keys;
        }
        let buffer = buffer.unwrap();

        let keypair = UserKeys::new(&buffer.private_key().to_hex_str().unwrap());
        if keypair.is_err() {
            return default_keys;
        }
        let keypair = Arc::new(keypair.unwrap());

        let public_key = keypair.get_public_key();
        let home_coordinates = extract_coordinates(&public_key).unwrap_or((0, 0, 0));
        let scaled_home_coordinates =
            scale_coordinates_to_world(home_coordinates.0, home_coordinates.1, home_coordinates.2);
        let home_coordinates = Vec3::new(
            scaled_home_coordinates.0 as f32,
            scaled_home_coordinates.1 as f32,
            scaled_home_coordinates.2 as f32,
        );

        UserNostrKeys {
            keypair,
            home_coordinates,
            public_key,
        }
    }
}

fn add_sample_blocks(
    mut commands: Commands,
    assets: Res<crate::resources::MeshesAndMaterials>,
    nostr_signer: Res<UserNostrKeys>,
) {
    // spawn a block of each type of material at my coordinate location
    let pubkey = nostr_signer.get_public_key();
    let home_coordinates = extract_coordinates(&pubkey).unwrap();
    let scale_coordinates =
        scale_coordinates_to_world(home_coordinates.0, home_coordinates.1, home_coordinates.2);
    let home_vec = Vec3::new(
        scale_coordinates.0,
        scale_coordinates.1,
        scale_coordinates.2,
    );

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.mud_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(0.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.bronze_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(1.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.iron_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(2.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.steel_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(3.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.mithril_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(4.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.adamant_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(5.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.rune_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(6.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();

    let _spawned_block = commands
        .spawn((PbrBundle {
            mesh: assets.cube_mesh.clone_weak(),
            material: assets.gold_material.clone_weak(),
            transform: Transform::from_translation(home_vec + Vec3::new(7.0, 1.0, 0.0)),
            ..Default::default()
        },))
        .id();
}
