use bevy::{
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    utils::{HashMap, HashSet},
};

use crate::{
    cyberspace::{extract_coordinates, scale_coordinates_to_world},
    nostr::POWBlockDetails,
};

pub const BRONZE: Color = Color::rgba_linear(0.804, 0.498, 0.196, 1.0);
pub const IRON: Color = Color::rgba_linear(0.435, 0.502, 0.564, 1.0);
pub const STEEL: Color = Color::rgba_linear(0.627, 0.627, 0.627, 1.0);
pub const MITHRIL: Color = Color::rgba_linear(0.482 * 10., 0.408 * 10., 0.776 * 10., 1.0);
pub const ADAMANT: Color = Color::rgba_linear(0.443 * 10., 0.651 * 10., 0.475 * 10., 1.0);
pub const RUNE: Color = Color::rgba_linear(0.416 * 10., 0.569 * 10., 0.824 * 10., 1.0);
pub const GOLD: Color = Color::rgba_linear(0.855 * 10., 0.647 * 10., 0.125 * 10., 1.0);

const STAR_COLOR: Color = Color::rgba_linear(1000.0, 1000., 1000., 0.01);

const BLOCK_SIZE: Vec3 = Vec3::splat(0.5);
const PUBKEY_SIZE: f32 = 1.0;

pub fn world_plugin(app: &mut App) {
    app.init_resource::<UniqueKeys>()
        .init_resource::<CoordinatesMap>()
        .add_systems(Startup, setup_world);
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
    asset_server: Res<AssetServer>,
) {
    // Add a light source
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0., f32::MAX, 0.)
            .looking_at(Vec3::new(-0.15, -0.05, 0.25), Vec3::Y),
        cascade_shadow_config,
        ..default()
    });

    // Load handles for reusable assets
    let cube_mesh = meshes.add(Mesh::from(Cuboid {
        half_size: BLOCK_SIZE,
        ..Default::default()
    }));
    let pubkey_mesh = meshes.add(Mesh::from(Sphere {
        radius: PUBKEY_SIZE,
        ..Default::default()
    }));

    let clear_material = materials.add(StandardMaterial {
        emissive: STAR_COLOR,
        alpha_mode: AlphaMode::Add,
        ..Default::default()
    });

    let clay_texture = asset_server.load("textures/clay.png");
    let mud_material = materials.add(StandardMaterial {
        base_color_texture: Some(clay_texture),
        metallic: 0.0,
        perceptual_roughness: 0.8,
        reflectance: 0.1,
        ..Default::default()
    });

    let bronze_texture = asset_server.load("textures/bronze.png");
    let bronze_material = materials.add(StandardMaterial {
        base_color_texture: Some(bronze_texture),
        emissive: BRONZE,
        metallic: 0.8,
        perceptual_roughness: 0.4,
        reflectance: 0.2,
        ..Default::default()
    });

    let iron_texture = asset_server.load("textures/iron.png");
    let iron_material = materials.add(StandardMaterial {
        base_color_texture: Some(iron_texture),
        emissive: IRON,
        metallic: 0.8,
        perceptual_roughness: 0.3,
        reflectance: 0.4,
        ..Default::default()
    });

    let steel_texture = asset_server.load("textures/steel.png");
    let steel_material = materials.add(StandardMaterial {
        base_color_texture: Some(steel_texture),
        emissive: STEEL,
        metallic: 0.9,
        perceptual_roughness: 0.2,
        reflectance: 0.8,
        ..Default::default()
    });

    let mithril_texture = asset_server.load("textures/mithril.png");
    let mithril_material = materials.add(StandardMaterial {
        base_color_texture: Some(mithril_texture),
        emissive: MITHRIL,
        metallic: 0.2,
        perceptual_roughness: 0.99,
        reflectance: 0.02,
        ior: 1.69,
        specular_transmission: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    let adamant_texture = asset_server.load("textures/adamant.png");
    let adamant_material = materials.add(StandardMaterial {
        base_color_texture: Some(adamant_texture),
        emissive: ADAMANT,
        metallic: 0.2,
        perceptual_roughness: 0.99,
        reflectance: 0.01,
        ior: 1.77,
        specular_transmission: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    let rune_texture = asset_server.load("textures/rune.png");
    let rune_material = materials.add(StandardMaterial {
        base_color_texture: Some(rune_texture),
        emissive: RUNE,
        metallic: 0.2,
        perceptual_roughness: 0.99,
        reflectance: 0.01,
        ior: 2.42,
        specular_transmission: 0.9,
        alpha_mode: AlphaMode::Blend,
        ..Default::default()
    });

    let gold_texture = asset_server.load("textures/gold.png");
    let gold_material = materials.add(StandardMaterial {
        base_color_texture: Some(gold_texture),
        emissive: GOLD,
        metallic: 0.9,
        perceptual_roughness: 0.1,
        reflectance: 0.9,
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
    block_details: &POWBlockDetails,
) -> Entity {
    let material = match block_details.pow_amount {
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

    let coordinates = block_details.coordinates();

    let spawned_block = commands
        .spawn((
            PbrBundle {
                mesh: stuff.cube_mesh.clone_weak(),
                material,
                transform: Transform::from_translation(coordinates),
                ..Default::default()
            },
            POWBlock {
                pow_amount: block_details.pow_amount,
                coordinate_string: block_details.coordinates.clone(),
                miner_pubkey: block_details.miner_pubkey.clone(),
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
    let (x, y, z) = extract_coordinates(&unique_key).unwrap();
    let (scaled_x, scaled_y, scaled_z) = scale_coordinates_to_world(x, y, z);

    commands.spawn(PbrBundle {
        mesh: stuff.pubkey_mesh.clone_weak(),
        material: stuff.clear_material.clone_weak(),
        transform: Transform::from_translation(Vec3::new(scaled_x, scaled_y, scaled_z)),
        ..Default::default()
    });
}
