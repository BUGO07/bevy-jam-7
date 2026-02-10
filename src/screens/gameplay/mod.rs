//! The screen state for the main gameplay.

use avian3d::{
    PhysicsPlugins,
    prelude::{CoefficientCombine, Collider, Friction, GravityScale, Restitution},
};
use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    input::common_conditions::input_just_pressed,
    light::{AtmosphereEnvironmentMapLight, SunDisk, VolumetricFog},
    pbr::{
        Atmosphere, AtmosphereSettings, Falloff, PhaseFunction, ScatteringMedium, ScatteringTerm,
    },
    post_process::bloom::Bloom,
    prelude::*,
    window::CursorOptions,
};
use bevy_seedling::sample::AudioSample;
use std::f32::consts::PI;

use crate::{
    Pause,
    asset_tracking::LoadResource,
    menus::Menu,
    screens::{Screen, gameplay::character_controller::CharacterControllerBundle, set_cursor_grab},
};

mod character_controller;
mod checkpoints;
mod enemy;

#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component)]
struct Player {
    // normalized values (0.0..1.0)
    health: f32,
    hallucination_severity: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            health: 1.0,
            hallucination_severity: 0.0,
        }
    }
}

impl Player {
    fn is_alive(&self) -> bool {
        self.health > 0.0
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        PhysicsPlugins::default(),
        character_controller::CharacterControllerPlugin,
        enemy::EnemyPlugin,
        checkpoints::CheckpointPlugin,
    ));
    app.load_resource::<LevelAssets>();
    app.add_systems(OnEnter(Screen::Gameplay), spawn_level);
    app.add_systems(
        OnExit(Screen::Gameplay),
        |mut commands: Commands, camera: Single<Entity, With<Camera3d>>| {
            commands.entity(*camera).remove_parent_in_place(); // make it so it's not despawned with the level
        },
    );

    // Toggle pause on key press.
    app.add_systems(
        Update,
        (
            (pause, spawn_background_overlay, open_pause_menu).run_if(
                in_state(Screen::Gameplay)
                    .and(in_state(Menu::None))
                    .and(input_just_pressed(KeyCode::Escape)),
            ),
            go_to_death_menu.run_if(in_state(Screen::Gameplay).and(in_state(Menu::None))),
        ),
    );
    app.add_systems(OnExit(Screen::Gameplay), (close_menu, unpause));
    app.add_systems(
        OnEnter(Menu::None),
        unpause.run_if(in_state(Screen::Gameplay)),
    );

    // Rotate sun
    app.add_systems(
        Update,
        update_sun.run_if(in_state(Screen::Gameplay).and(in_state(Pause(false)))),
    );
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency]
    music: Handle<AudioSample>,
    #[dependency]
    cube: Handle<Scene>,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/Fluffing A Duck.ogg"),
            cube: assets.load(GltfAssetLabel::Scene(0).from_asset("models/scene.glb")),
        }
    }
}

fn go_to_death_menu(
    mut commands: Commands,
    mut next_menu: ResMut<NextState<Menu>>,
    mut paused: ResMut<NextState<Pause>>,
    player: Single<&Player>,
) {
    if !player.is_alive() {
        commands.run_system_cached(spawn_background_overlay);
        next_menu.set(Menu::Death);
        paused.set(Pause(true));
    }
}

fn spawn_level(
    mut commands: Commands,
    level_assets: Res<LevelAssets>,
    camera: Single<Entity, With<Camera3d>>,
    mut cursor_options: Single<&mut CursorOptions>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    set_cursor_grab(&mut cursor_options, true);
    let player = commands
        .spawn((
            Name::new("Player"),
            CharacterControllerBundle::new(Collider::capsule(0.4, 1.0)).with_movement(
                75.0,
                0.92,
                7.0,
                35f32.to_radians(),
            ),
            Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            GravityScale(2.0),
            Transform::from_xyz(0.0, 1.8, 2.0),
            Player {
                health: 1.0,
                hallucination_severity: 0.0,
            },
        ))
        .add_child(*camera)
        .id();

    let music = commands
        .spawn((
            Name::new("Gameplay Music"),
            // music(level_assets.music.clone()),
        ))
        .id();

    // Set camera position and add atmosphere
    commands.entity(*camera).insert((
        Transform::from_xyz(0.0, 0.8, 0.0),
        Atmosphere::earthlike(scattering_mediums.add(
            // Custom alien green atmosphere with green-dominant Rayleigh scattering
            ScatteringMedium::new(
                256,
                256,
                [
                    // Rayleigh scattering term - green-purple
                    ScatteringTerm {
                        absorption: Vec3::ZERO,
                        scattering: Vec3::new(15.0e-6, 80.0e-6, 10.0e-6), // Much higher green, reduced red/blue
                        falloff: Falloff::Exponential { scale: 9.0 / 60.0 },
                        phase: PhaseFunction::Rayleigh,
                    },
                    // Mie scattering term - thick atmosphere (2.0e-6)
                    ScatteringTerm {
                        absorption: Vec3::splat(1.996e-6),
                        scattering: Vec3::splat(1.68e-6), // Thick Mie scattering
                        falloff: Falloff::Exponential { scale: 1.2 / 60.0 },
                        phase: PhaseFunction::Mie { asymmetry: 0.8 },
                    },
                ],
            ),
        )),
        AtmosphereSettings::default(),
        Exposure {
            ev100: Exposure::EV100_BLENDER,
        },
        Tonemapping::AcesFitted,
        Bloom::NATURAL,
        AtmosphereEnvironmentMapLight::default(),
        VolumetricFog::default(),
    ));

    let light = commands
        .spawn((
            Name::new("Light"),
            DirectionalLight {
                shadows_enabled: true,
                // Light blue light
                color: Color::LinearRgba(LinearRgba::new(0.2, 0.5, 1.0, 1.0)),
                ..default()
            },
            SunDisk {
                angular_size: 0.06,
                intensity: 0.5,
            },
            Transform::from_rotation(Quat::from_euler(
                EulerRot::YXZ,
                -35f32.to_radians(),
                -25f32.to_radians(),
                0.0,
            )),
        ))
        .id();

    let level = commands
        .spawn((
            Name::new("Level"),
            Transform::default(),
            Visibility::default(),
            DespawnOnExit(Screen::Gameplay),
            SceneRoot(level_assets.cube.clone()),
        ))
        .add_children(&[player, light, music])
        .id();

    commands.queue(enemy::EnemySpawnCmd {
        pos: Isometry3d::from_translation(vec3(0.0, 0.9, 5.0)),
        parent: Some(level),
    });
    commands.queue(enemy::EnemySpawnCmd {
        pos: Isometry3d::from_translation(vec3(4.0, 0.9, 5.0)),
        parent: Some(level),
    });
}

fn unpause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(false));
}

fn pause(mut next_pause: ResMut<NextState<Pause>>) {
    next_pause.set(Pause(true));
}

fn spawn_background_overlay(mut commands: Commands) {
    commands.spawn((
        Name::new("Background Overlay"),
        Node {
            width: percent(100),
            height: percent(100),
            ..default()
        },
        GlobalZIndex(1),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        DespawnOnExit(Pause(true)),
    ));
}

fn open_pause_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::Pause);
}

fn close_menu(mut next_menu: ResMut<NextState<Menu>>) {
    next_menu.set(Menu::None);
}

fn update_sun(mut suns: Query<&mut Transform, With<DirectionalLight>>, time: Res<Time>) {
    // TODO: tweak movement and speed of the sun
    // currently rotates
    suns.iter_mut()
        .for_each(|mut tf| tf.rotate_x(-time.delta_secs() * PI / 100.0));
}
