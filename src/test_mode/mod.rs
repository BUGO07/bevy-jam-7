use bevy::prelude::*;
use crate::screens::Screen;

pub struct TestModePlugin;

impl Plugin for TestModePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<TestModeActive>()
            .add_systems(OnEnter(Screen::Test), setup_test)
            .add_systems(Update, rotate_cube.run_if(in_state(Screen::Test)))
            .add_systems(Update, control_test_camera.run_if(in_state(Screen::Test)))
            .add_systems(Update, toggle_cameras)
            .add_systems(OnExit(Screen::Test), cleanup_test);
    }
}

// ---------------- COMPONENTS ----------------
#[derive(Component)]
struct TestEntity;

#[derive(Component)]
struct RotatingCube;

#[derive(Component)]
struct TestModeCamera;

// Make MainCamera public so it's accessible from main.rs
#[derive(Component)]
pub struct MainCamera;

// ---------------- RESOURCES ----------------
#[derive(Resource, Default)]
struct TestModeActive(pub bool);

// ---------------- SETUP ----------------
fn setup_test(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Entered TEST mode");

    // Light
    commands.spawn((
        PointLight {
            intensity: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
        TestEntity,
    ));

    // Cube - use meshes and materials as handles
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.48, 0.56, 1.0),
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 0.0),
        RotatingCube,
        TestEntity,
    ));

    // Test Mode Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 3.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        TestModeCamera,
        TestEntity,
    ));

    commands.insert_resource(TestModeActive(true));
}

// ---------------- ROTATION ----------------
fn rotate_cube(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<RotatingCube>>,
) {
    for mut transform in &mut query {
        let delta = Quat::from_euler(
            EulerRot::XYZ,
            0.8 * time.delta_secs(),
            1.5 * time.delta_secs(),
            0.0
        );
        transform.rotation = delta * transform.rotation;
    }
}

// ---------------- CAMERA CONTROL ----------------
fn control_test_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<TestModeCamera>>,
    time: Res<Time>,
) {
    for mut transform in &mut query {
        let mut dir = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) { dir.z -= 1.0; }
        if keyboard.pressed(KeyCode::KeyS) { dir.z += 1.0; }
        if keyboard.pressed(KeyCode::KeyA) { dir.x -= 1.0; }
        if keyboard.pressed(KeyCode::KeyD) { dir.x += 1.0; }
        let speed = 5.0 * time.delta_secs();
        let rotation = transform.rotation;
        transform.translation += rotation * dir * speed;
    }
}

// ---------------- TOGGLE CAMERAS ----------------
fn toggle_cameras(
    test_mode: Res<TestModeActive>,
    mut main_cam_query: Query<&mut Camera, (With<MainCamera>, Without<TestModeCamera>)>,
    mut test_cam_query: Query<&mut Camera, (With<TestModeCamera>, Without<MainCamera>)>,
) {
    for mut cam in &mut main_cam_query {
        cam.is_active = !test_mode.0;
    }
    for mut cam in &mut test_cam_query {
        cam.is_active = test_mode.0;
    }
}

// ---------------- CLEANUP ----------------
fn cleanup_test(
    mut commands: Commands,
    query: Query<Entity, With<TestEntity>>,
    mut test_mode: ResMut<TestModeActive>,
) {
    info!("Exited TEST mode");
    for e in &query {
        commands.entity(e).despawn();
    }
    test_mode.0 = false;
}
