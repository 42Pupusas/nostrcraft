use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};



use crate::{
    cameras::ExplorerCameraBundle,
    cyberspace::{hex_string_to_i_space, i_space_to_hex_string},
    nostr::POWBlockDetails,
};

pub const MARGINS_UI: UiRect = UiRect::all(Val::Percent(2.1));

pub const TERRACOTA: &str = "E26F57";
pub const BRONZE: Color = Color::rgba(0.804, 0.498, 0.196, 1.0);
pub const IRON: Color = Color::rgba(0.435, 0.502, 0.564, 1.0);
pub const STEEL: Color = Color::rgba(0.627, 0.627, 0.627, 1.0);
pub const MITHRIL: Color = Color::rgba(0.482, 0.408, 0.776, 1.0);
pub const ADAMANT: Color = Color::rgba(0.443, 0.651, 0.475, 1.0);
pub const RUNE: Color = Color::rgba(0.416, 0.569, 0.824, 1.0);
pub const GOLD: Color = Color::rgba(0.855, 0.647, 0.125, 1.0);

pub fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn_recursive();
    }
}

pub const FONT_SIZE_UI_TITLES: f32 = 24.0;
pub const FONT_SIZE_UI: f32 = 20.0;
pub const FONT_SIZE_UI_SMALL: f32 = 16.0;

#[derive(Component)]
pub struct BlockIndicator;

#[derive(Component)]
struct ColorText;

pub fn world_plugin(app: &mut App) {
    app.init_resource::<UniqueKeys>()
        .init_resource::<CoordinatesMap>()
        .add_systems(Startup, setup_world)
        .add_systems(Update, (display_coordinates, move_block_indicator));
}

// KEY 55BE2A31916E238A5D21F44DEAF7FA2579D11EEEB98D022842A15A2C7AF2F106

#[derive(Resource, Deref, DerefMut, Debug)]
pub struct UniqueKeys(pub HashSet<String>);

impl Default for UniqueKeys {
    fn default() -> Self {
        UniqueKeys(HashSet::new())
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct CoordinatesMap(pub HashMap<String, POWBlockDetails>);

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
    }));

    let clear_material = materials.add(StandardMaterial {
        emissive: Color::rgba_linear(100.0, 100., 100., 0.01),
        alpha_mode: AlphaMode::Add,
        ..Default::default()
    });

    let pubkey_mesh = meshes.add(Mesh::from(Sphere { radius: 21.0 }));

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

    let location = Vec3::new(0.0, 10.0, -10.0);
    let look_at = Vec3::ZERO;

    // Setup a ghot block to indicate the player's location
    // And a camera as a child so it will orbit the block
    commands
        .spawn((
            PbrBundle {
                mesh: cube_mesh.clone_weak(),
                material: clear_material.clone_weak(),
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ..Default::default()
            },
            BlockIndicator,
        ))
        .with_children(|builder| {
            builder.spawn(ExplorerCameraBundle::new_default(location, look_at));
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

    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "hello\nbevy!",
            TextStyle {
                font_size: 21.0,
                ..default()
            },
        ) // Set the justification of the Text
        .with_text_justify(JustifyText::Center)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        }),
        ColorText,
    ));
}
#[derive(Bundle)]
pub struct UiSection {
    pub node: NodeBundle,
    pub title: TextBundle,
    pub value: TextBundle,
}

impl UiSection {
    pub fn new(value: &str) -> Self {
        UiSection {
            node: Self::ui_value_node(),
            title: Self::ui_value_title_section(value),
            value: Self::ui_value_text_section(),
        }
    }

    pub fn new_title(value: &str) -> Self {
        UiSection {
            node: Self::ui_title_node(),
            title: Self::ui_group_title(value),
            value: TextBundle::default(),
        }
    }

    fn ui_title_node() -> NodeBundle {
        NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                border: UiRect::bottom(Val::Px(2.)),
                margin: UiRect::bottom(Val::Px(4.)),
                width: Val::Percent(80.0),
                ..Default::default()
            },
            border_color: BorderColor(Color::WHITE),
            ..Default::default()
        }
    }

    fn ui_group_title(value: &str) -> TextBundle {
        TextBundle::from_section(
            value.to_string(),
            TextStyle {
                font_size: FONT_SIZE_UI_TITLES,
                color: Color::WHITE,
                ..Default::default()
            },
        )
    }

    fn ui_value_node() -> NodeBundle {
        NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                width: Val::Percent(80.0),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ui_value_title_section(value: &str) -> TextBundle {
        TextBundle::from_section(
            value.to_string(),
            TextStyle {
                font_size: FONT_SIZE_UI,
                color: Color::WHITE,
                ..Default::default()
            },
        )
    }

    fn ui_value_text_section() -> TextBundle {
        TextBundle::from_section(
            "".to_string(),
            TextStyle {
                font_size: FONT_SIZE_UI_SMALL,
                color: Color::WHITE,
                ..Default::default()
            },
        )
    }
}

fn display_coordinates(
    query: Query<&Transform, With<BlockIndicator>>,
    mut text_query: Query<&mut Text>,
) {
    // need to log the coordinates of the mouse click
    let transform = query.single();
    let x = transform.translation.x;
    let y = transform.translation.y;
    let z = transform.translation.z;
    for mut text in text_query.iter_mut() {
        text.sections[0].value = format!("x: {}\ny: {}\nz: {}", x, y, z);
    }
}

fn move_block_indicator(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &BlockIndicator)>,
) {
    for (mut transform, _block_indicator) in query.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::KeyW) {
            transform.translation.z -= 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyS) {
            transform.translation.z += 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyA) {
            transform.translation.x -= 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyD) {
            transform.translation.x += 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyQ) {
            transform.translation.y += 1.0;
        }
        if keyboard_input.just_pressed(KeyCode::KeyE) {
            transform.translation.y -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowUp) {
            transform.translation.z -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowDown) {
            transform.translation.z += 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::ArrowRight) {
            transform.translation.x += 1.0;
        }

        if keyboard_input.pressed(KeyCode::PageUp) {
            transform.translation.y += 1.0;
        }

        if keyboard_input.pressed(KeyCode::PageDown) {
            transform.translation.y -= 1.0;
        }
    }
}

#[derive(Component, Clone)]
pub struct POWBlock {
    pow_amount: usize,
    coordinate_string: String,
    miner_pubkey: String,
}

pub fn spawn_mined_block(
    commands: &mut Commands,
    stuff: &Res<MeshesAndMaterials>,
    coordinates: Vec3,
    pow_amount: usize,
    miner_pubkey: String,
) {
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

    commands.spawn((
        PbrBundle {
            mesh: stuff.cube_mesh.clone_weak(),
            material,
            transform: Transform::from_translation(coordinates),
            ..Default::default()
        },
        POWBlock {
            pow_amount,
            coordinate_string: i_space_to_hex_string(coordinates.x, coordinates.y, coordinates.z),
            miner_pubkey,
        },
    ));
}

pub fn spawn_pubkey_note(
    commands: &mut Commands,
    stuff: &Res<MeshesAndMaterials>,
    unique_key: String,
) {
    info!("Spawning pubkey note at: {}", unique_key);
    let (x, y, z) = hex_string_to_i_space(&unique_key);
    let x = x % 10000.0;
    let y = y % 10000.0;
    let z = z % 10000.0;
    info!("Spawning pubkey note at: {} {} {}", x, y, z);

    commands.spawn(PbrBundle {
        mesh: stuff.pubkey_mesh.clone_weak(),
        material: stuff.clear_material.clone_weak(),
        transform: Transform::from_translation(Vec3::new(x, y, z)),
        ..Default::default()
    });
}
