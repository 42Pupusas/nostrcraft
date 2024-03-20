use super::resources::{despawn_screen, UiSection, MARGINS_UI};
use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    input::mouse::MouseMotion,
    prelude::*,
    render::camera::RenderTarget,
    window::WindowRef,
};

#[derive(Bundle)]
pub struct UiCameraBundle {
    camera_entity: Camera2dBundle,
    camera_component: UiCamera,
    screen_node: NodeBundle,
    screen_component: UiNode,
    target: TargetCamera,
}

#[derive(Component)]
pub struct UiCamera;

#[derive(Component)]
pub struct UiNode;

impl UiCameraBundle {
    pub fn new(window: WindowRef, commands: &mut Commands) {
        let camera_bundle = Camera2dBundle {
            camera: Camera {
                target: RenderTarget::Window(window),
                ..Default::default()
            },
            ..Default::default()
        };

        let camera_entity = commands.spawn((camera_bundle, UiCamera)).id();

        let screen_bundle = NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                margin: MARGINS_UI,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceEvenly,
                align_items: AlignItems::FlexStart,
                flex_wrap: FlexWrap::Wrap,
                ..Default::default()
            },
            ..Default::default()
        };
        commands.spawn((screen_bundle, UiNode, TargetCamera(camera_entity)));
    }
}

#[derive(Component)]
pub enum CameraLog {
    Location,
    LookingAt,
    Sensitivity,
    Orbiting,
    OrbitSpeed,
}

#[derive(Component)]
struct CameraLogNode;

fn setup_camera_ui(mut commands: Commands, mut ui_node: Query<Entity, With<UiNode>>) {
    if let Ok(ui_node) = ui_node.get_single_mut() {
        commands.entity(ui_node).with_children(|ui_screen| {
            ui_screen
                .spawn((
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceEvenly,
                            align_items: AlignItems::Center,
                            width: Val::Percent(40.),
                            margin: MARGINS_UI,
                            ..default()
                        },
                        ..Default::default()
                    },
                    CameraLogNode,
                ))
                .with_children(|parent| {
                    let ui_section_title = UiSection::new_title("Camera Details");
                    parent
                        .spawn(ui_section_title.node)
                        .with_children(|data_row| {
                            data_row.spawn(ui_section_title.title);
                        });

                    let ui_section_location = UiSection::new("Location");
                    parent
                        .spawn(ui_section_location.node)
                        .with_children(|data_row| {
                            data_row.spawn(ui_section_location.title);
                            data_row.spawn((ui_section_location.value, CameraLog::Location));
                        });

                    let ui_section_looking_at = UiSection::new("Looking At");
                    parent
                        .spawn(ui_section_looking_at.node)
                        .with_children(|data_row| {
                            data_row.spawn(ui_section_looking_at.title);
                            data_row.spawn((ui_section_looking_at.value, CameraLog::LookingAt));
                        });

                    let ui_section_sensitivity = UiSection::new("Sensitivity");
                    parent
                        .spawn(ui_section_sensitivity.node)
                        .with_children(|data_row| {
                            data_row.spawn(ui_section_sensitivity.title);
                            data_row.spawn((ui_section_sensitivity.value, CameraLog::Sensitivity));
                        });

                    let ui_section_orbiting = UiSection::new("Orbiting");
                    parent
                        .spawn(ui_section_orbiting.node)
                        .with_children(|data_row| {
                            data_row.spawn(ui_section_orbiting.title);
                            data_row.spawn((ui_section_orbiting.value, CameraLog::Orbiting));
                        });

                    let ui_section_orbit_speed = UiSection::new("Orbit Speed");
                    parent
                        .spawn(ui_section_orbit_speed.node)
                        .with_children(|data_row| {
                            data_row.spawn(ui_section_orbit_speed.title);
                            data_row.spawn((ui_section_orbit_speed.value, CameraLog::OrbitSpeed));
                        });
                });
        });
    }
}

pub fn camera_debug_system(
    active_camera_query: Query<(&CameraState, &Transform), Without<UiCamera>>,
    mut debug_camera_query: Query<(&mut Text, &CameraLog), With<CameraLog>>,
) {
    if let Ok((active_camera_state, active_camera_transform)) = active_camera_query.get_single() {
        for (mut text, log) in debug_camera_query.iter_mut() {
            match log {
                CameraLog::Location => {
                    text.sections[0].value = format!(
                        "x: {:.0} y: {:.0} z: {:.0}",
                        active_camera_transform.translation.x,
                        active_camera_transform.translation.y,
                        active_camera_transform.translation.z
                    );
                }
                CameraLog::LookingAt => {
                    text.sections[0].value = format!(
                        "x: {:.0} y: {:.0} z: {:.0}",
                        active_camera_transform.rotation.mul_vec3(Vec3::Z).x,
                        active_camera_transform.rotation.mul_vec3(Vec3::Z).y,
                        active_camera_transform.rotation.mul_vec3(Vec3::Z).z
                    );
                }
                CameraLog::Sensitivity => {
                    text.sections[0].value = format!("{:.2}", active_camera_state.sensitivity);
                }
                CameraLog::Orbiting => {
                    text.sections[0].value = format!("{}", active_camera_state.is_orbiting);
                }
                CameraLog::OrbitSpeed => {
                    text.sections[0].value = format!("{:.2}", active_camera_state.orbit_speed);
                }
            }
        }
    }
}

#[derive(Bundle)]
pub struct ExplorerCameraBundle(Camera3dBundle, ExplorerCamera, BloomSettings, CameraState);

#[derive(Component)]
pub struct ExplorerCamera;

#[derive(Component, Copy, Clone, Debug)]
pub struct CameraState {
    pub speed: f32,
    pub acceleration: f32,
    pub acceleration_time: f32,
    pub is_orbiting: bool, // New field to indicate if the camera is orbiting
    pub focus_point: Quat, // New field to store the focus point
    pub orbit_speed: f32,
    pub sensitivity: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        CameraState {
            speed: 2100.0,
            acceleration: 9.8,
            acceleration_time: 0.0,
            focus_point: Quat::IDENTITY,
            is_orbiting: false,
            orbit_speed: 0.1,
            sensitivity: 21.0,
        }
    }
}

impl CameraState {
    pub fn move_forward(&self, time: f32) -> Vec3 {
        let forward = self.focus_point.mul_vec3(Vec3::Z).normalize();
        forward * self.speed * time
    }

    pub fn move_backward(&self, time: f32) -> Vec3 {
        let backward = self.focus_point.mul_vec3(Vec3::Z).normalize();
        backward * self.speed * time
    }

    pub fn move_left(&self, time: f32) -> Vec3 {
        let left = self.focus_point.mul_vec3(Vec3::X).normalize();
        left * self.speed * time
    }

    pub fn move_right(&self, time: f32) -> Vec3 {
        let right = self.focus_point.mul_vec3(Vec3::X).normalize();
        right * self.speed * time
    }

    pub fn move_up(&self, time: f32) -> Vec3 {
        let up = self.focus_point.mul_vec3(Vec3::Y).normalize();
        up * self.speed * time
    }

    pub fn move_down(&self, time: f32) -> Vec3 {
        let down = self.focus_point.mul_vec3(Vec3::Y).normalize();
        down * self.speed * time
    }
}

impl ExplorerCameraBundle {
    pub fn new_default(location: Vec3, looking_at: Vec3) -> Self {
        let camera_entity = Camera3dBundle {
            camera: Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                target: RenderTarget::Window(WindowRef::Primary),
                ..Default::default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            transform: Transform::from_translation(location).looking_at(looking_at, Vec3::Y),
            ..Default::default()
        };

        ExplorerCameraBundle {
            0: camera_entity,
            1: ExplorerCamera,
            2: BloomSettings {
                intensity: 0.21,

                ..Default::default()
            },
            3: CameraState::default(),
        }
    }

    pub fn new_with_changes(
        location: Vec3,
        looking_at: Vec3,
        camera: Camera,
        bloom: BloomSettings,
    ) -> Self {
        let camera_entity = Camera3dBundle {
            camera,
            transform: Transform::from_translation(location).looking_at(looking_at, Vec3::Y),
            ..Default::default()
        };

        ExplorerCameraBundle {
            0: camera_entity,
            1: ExplorerCamera,
            2: bloom,
            3: CameraState::default(),
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum ExplorerCameraDebugger {
    Inactive,
    #[default]
    Active,
}

pub fn camera_plugin(app: &mut App) {
    app.add_systems(OnEnter(ExplorerCameraDebugger::Active), setup_camera_ui)
        .add_systems(
            Update,
            (
                camera_look_system,
                camera_displacement,
                //camera_orbit_system,
                camera_debug_system,
            )
                .run_if(in_state(ExplorerCameraDebugger::Active)),
        )
        .add_systems(
            OnExit(ExplorerCameraDebugger::Active),
            (despawn_screen::<CameraLogNode>,),
        );
}

pub fn camera_displacement(
    _time: Res<Time>,
    _keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_state: Query<&mut Transform, With<Camera>>,
) {
    if let Ok(_camera_transform) = camera_state.get_single_mut() {}
}

pub fn camera_look_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut camera_state: Query<(&mut CameraState, &mut Transform)>,
) {
    if let Ok((_camera_state, mut camera_transform)) = camera_state.get_single_mut() {
        let vec_forward = camera_transform.rotation.mul_vec3(Vec3::Z);

        if mouse_input.pressed(MouseButton::Right) {
            let delta: Vec2 = mouse_motion_events
                .read()
                .fold(Vec2::ZERO, |acc, motion| acc + motion.delta);
            // Calculate the pitch adjustment relative to the camera's current orientation
            let right_dir = camera_transform.local_x();
            let pitch_quat = Quat::from_axis_angle(*right_dir, -delta.y * 0.01);
            camera_transform.rotate_around(Vec3::ZERO, pitch_quat);

            // Move the yaw with delta.x
            camera_transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(delta.x * 0.01));
        }

        if mouse_input.pressed(MouseButton::Middle) {
            let delta: Vec2 = mouse_motion_events
                .read()
                .fold(Vec2::ZERO, |acc, motion| acc + motion.delta);
            camera_transform.translation += vec_forward * delta.y * 0.1;
        }
    }
}
