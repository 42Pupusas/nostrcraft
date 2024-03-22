use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use nostro2::userkeys::UserKeys;

use crate::{
    cyberspace::{extract_coordinates, scale_coordinates_to_world},
    nostr::POWBlockDetails,
};

pub const TERRACOTA: &str = "E26F57";
pub const BRONZE: Color = Color::rgba(0.804, 0.498, 0.196, 1.0);
pub const IRON: Color = Color::rgba(0.435, 0.502, 0.564, 1.0);
pub const STEEL: Color = Color::rgba(0.627, 0.627, 0.627, 1.0);
pub const MITHRIL: Color = Color::rgba(0.482, 0.408, 0.776, 1.0);
pub const ADAMANT: Color = Color::rgba(0.443, 0.651, 0.475, 1.0);
pub const RUNE: Color = Color::rgba(0.416, 0.569, 0.824, 1.0);
pub const GOLD: Color = Color::rgba(0.855, 0.647, 0.125, 1.0);

pub fn world_plugin(app: &mut App) {
    app.init_resource::<NostrSigner>()
        .init_resource::<UniqueKeys>()
        .init_resource::<CoordinatesMap>()
        .add_systems(Startup, setup_world);
}

// KEY 55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106

#[derive(Resource, Deref, DerefMut)]
pub struct NostrSigner(pub UserKeys);

impl Default for NostrSigner {
    fn default() -> Self {
        NostrSigner(
            UserKeys::new("55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106")
                .unwrap(),
        )
    }
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct UniqueKeys(pub HashSet<String>);

impl Default for UniqueKeys {
    fn default() -> Self {
        UniqueKeys(HashSet::new())
    }
}

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct CoordinatesMap(pub HashMap<String, (Entity, POWBlockDetails)>);

impl Default for CoordinatesMap {
    fn default() -> Self {
        CoordinatesMap(HashMap::new())
    }
}

#[derive(Resource)]
pub struct MeshesAndMaterials {
    pub pubkey_mesh: Handle<Mesh>,
    pub cube_mesh: Handle<Mesh>,
    pub clear_material: Handle<StandardMaterial>,
    pub mud_material: Handle<StandardMaterial>,
    pub bronze_material: Handle<StandardMaterial>,
    pub iron_material: Handle<StandardMaterial>,
    pub steel_material: Handle<StandardMaterial>,
    pub mithril_material: Handle<StandardMaterial>,
    pub adamant_material: Handle<StandardMaterial>,
    pub rune_material: Handle<StandardMaterial>,
    pub gold_material: Handle<StandardMaterial>,
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Load handles for reusable assets
    let cube_mesh = meshes.add(Mesh::from(Cuboid {
        half_size: Vec3::splat(0.5),
        ..Default::default()
    }));
    let pubkey_mesh = meshes.add(Mesh::from(Sphere {
        radius: 0.5,
        ..Default::default()
    }));

    let clear_material = materials.add(StandardMaterial {
        emissive: Color::rgba_linear(1000.0, 1000., 1000., 0.01),
        alpha_mode: AlphaMode::Add,
        ..Default::default()
    });

    let mud_material = materials.add(StandardMaterial {
        base_color: Color::hex(TERRACOTA).unwrap(),
        ..Default::default()
    });

    let bronze_material = materials.add(StandardMaterial {
        base_color: BRONZE,
        emissive: BRONZE,
        ..Default::default()
    });

    let iron_material = materials.add(StandardMaterial {
        base_color: IRON,
        emissive: IRON,
        ..Default::default()
    });

    let steel_material = materials.add(StandardMaterial {
        base_color: STEEL,
        emissive: STEEL,
        ..Default::default()
    });

    let mithril_material = materials.add(StandardMaterial {
        base_color: MITHRIL,
        emissive: MITHRIL,
        ..Default::default()
    });

    let adamant_material = materials.add(StandardMaterial {
        base_color: ADAMANT,
        emissive: ADAMANT,
        ..Default::default()
    });

    let rune_material = materials.add(StandardMaterial {
        base_color: RUNE,
        emissive: RUNE,
        ..Default::default()
    });

    let gold_material = materials.add(StandardMaterial {
        base_color: GOLD,
        emissive: GOLD,
        ..Default::default()
    });

    commands.insert_resource(MeshesAndMaterials {
        pubkey_mesh,
        cube_mesh,
        clear_material,
        mud_material,
        bronze_material,
        iron_material,
        steel_material,
        mithril_material,
        adamant_material,
        rune_material,
        gold_material,
    });
}

#[derive(Component, Clone)]
pub struct POWBlock {
    pub pow_amount: usize,
    pub coordinate_string: String,
    pub miner_pubkey: String,
}

pub fn spawn_mined_block(
    commands: &mut Commands,
    stuff: &Res<MeshesAndMaterials>,
    coordinates: Vec3,
    pow_amount: usize,
    miner_pubkey: String,
) -> Entity {
    let material = match pow_amount {
        0 => stuff.mud_material.clone_weak(),
        1 => stuff.mud_material.clone_weak(),
        2 => stuff.bronze_material.clone_weak(),
        3 => stuff.iron_material.clone_weak(),
        4 => stuff.steel_material.clone_weak(),
        5 => stuff.mithril_material.clone_weak(),
        6 => stuff.adamant_material.clone_weak(),
        7 => stuff.rune_material.clone_weak(),
        _ => stuff.gold_material.clone_weak(),
    };

    let spawned_block = commands
        .spawn((
            PbrBundle {
                mesh: stuff.cube_mesh.clone_weak(),
                material,
                transform: Transform::from_translation(coordinates),
                ..Default::default()
            },
            POWBlock {
                pow_amount,
                coordinate_string: coordinates.to_string(),
                miner_pubkey,
            },
        ))
        .id();
    spawned_block
}

pub fn spawn_pubkey_note(
    commands: &mut Commands,
    stuff: &Res<MeshesAndMaterials>,
    unique_key: String,
) {
    info!("Spawning pubkey note at: {}", unique_key);
    let (x, y, z) = extract_coordinates(&unique_key).unwrap();
    let (scaled_x, scaled_y, scaled_z) = scale_coordinates_to_world(x, y, z);
    info!(
        "Scaled coordinates: x: {} y: {} z: {}",
        scaled_x, scaled_y, scaled_z
    );

    commands.spawn(PbrBundle {
        mesh: stuff.pubkey_mesh.clone_weak(),
        material: stuff.clear_material.clone_weak(),
        transform: Transform::from_translation(Vec3::new(scaled_x, scaled_y, scaled_z)),
        ..Default::default()
    });
}
