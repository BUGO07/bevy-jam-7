//! The screen state for the main gameplay.

use avian3d::prelude::*;
use bevy::{
    anti_alias::fxaa::Fxaa,
    camera::Exposure,
    core_pipeline::{Skybox, tonemapping::Tonemapping},
    input::common_conditions::input_just_pressed,
    post_process::bloom::Bloom,
    prelude::*,
    window::CursorOptions,
};
use bevy_landmass::prelude::*;
use bevy_rerecast::prelude::*;
use bevy_seedling::sample::AudioSample;
use landmass_rerecast::{Island3dBundle, LandmassRerecastPlugin, NavMeshHandle3d};
use std::f32::consts::PI;

use crate::{
    PausableSystems, Pause,
    asset_tracking::LoadResource,
    menus::Menu,
    screens::{
        Screen,
        gameplay::{
            character_controller::CameraRotation,
            hammerhead::HammerheadAssets,
            katana::{katana_animation, katana_setup, poor_setup_for_katana_animations},
            player::Player,
        },
        set_cursor_grab,
    },
};

mod character_controller;
mod checkpoints;
mod enemy;
mod hammerhead;
mod katana;
mod player;

#[derive(Component)]
struct Level;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        PhysicsPlugins::default(),
        bevy_landmass::Landmass3dPlugin::default(),
        bevy_landmass::debug::Landmass3dDebugPlugin::default(),
        bevy_rerecast::NavmeshPlugins::default(),
        avian_rerecast::AvianBackendPlugin::default(),
        LandmassRerecastPlugin::default(),
        character_controller::CharacterControllerPlugin,
        enemy::EnemyPlugin,
        checkpoints::CheckpointPlugin,
        hammerhead::hammerhead,
    ));
    app.load_resource::<LevelAssets>();
    app.add_systems(
        OnEnter(Screen::Gameplay),
        (spawn_level, katana_setup).chain(),
    );
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

    // todo: system ordering is likely incorrect and use FIXED UPDATE here.
    app.add_systems(
        Update,
        generate_navmesh.run_if(in_state(Screen::Gameplay)), //.run_if(input_just_pressed(KeyCode::Space)),
    );
    app.add_observer(handle_navmesh_ready);
    app.add_systems(
        Update,
        (poor_setup_for_katana_animations, katana_animation)
            .in_set(PausableSystems)
            .run_if(in_state(Screen::Gameplay)),
    );
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
pub struct LevelAssets {
    #[dependency] music: Handle<AudioSample>,
    #[dependency] step1: Handle<AudioSample>,
    #[dependency] whoosh1: Handle<AudioSample>,
    #[dependency] demo_level: Handle<Scene>,
    #[dependency] skybox: Handle<Image>,
    #[dependency] katana_idle: Handle<AnimationClip>,
    #[dependency] katana_swing: Handle<AnimationClip>,
    #[dependency] katana_scene: Handle<Scene>,
    #[dependency] hammerhead: HammerheadAssets,
}

impl FromWorld for LevelAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/Fluffing A Duck.ogg"),
            step1: assets.load("audio/sound_effects/step1.wav"),
            whoosh1: assets.load("audio/sound_effects/whoosh1.wav"),
            demo_level: assets.load(GltfAssetLabel::Scene(1).from_asset("models/Demo_level_heaven_sword.glb")),
            skybox: assets.load("images/skybox.ktx2"),
            katana_idle: assets.load(GltfAssetLabel::Animation(0).from_asset("models/katana.glb")),
            katana_swing: assets.load(GltfAssetLabel::Animation(1).from_asset("models/katana.glb")),
            katana_scene: assets.load(GltfAssetLabel::Scene(0).from_asset("models/katana.glb")),
            hammerhead: HammerheadAssets::load(assets),
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
    mut generator: NavmeshGenerator,
) {
    commands.insert_resource(NavmeshDone(false));
    let camera = *camera;

    let archipelago_options: ArchipelagoOptions<ThreeD> =
        ArchipelagoOptions::from_agent_radius(0.5);
    // archipelago_options.point_sample_distance.distance_above = -2.5;
    // archipelago_options.point_sample_distance.distance_below = -2.5;

    let archipelago_id = commands.spawn(Archipelago3d::new(archipelago_options)).id();

    commands.spawn(Island3dBundle {
        island: Island,
        archipelago_ref: ArchipelagoRef3d::new(archipelago_id),
        nav_mesh: NavMeshHandle3d(generator.generate(NavmeshSettings {
            agent_radius: 0.5,
            ..default()
        })),
    });

    commands.insert_resource(NavmeshArchipelagoHolder(archipelago_id));

    set_cursor_grab(&mut cursor_options, true);
    let player = player::spawn_player(&mut commands, camera);

    let music = commands
        .spawn((
            Name::new("Gameplay Music"),
            // music(level_assets.music.clone()),
        ))
        .id();

    // Set camera position and add atmosphere
    let transform = Transform::from_xyz(0.0, 0.8 + 0.9, 0.0);
    commands.entity(camera).insert((
        transform,
        CameraRotation(transform.rotation.x),
        Skybox {
            image: level_assets.skybox.clone(),
            brightness: 1000.0,
            ..Default::default()
        },
    ));

    let light = commands
        .spawn((
            Name::new("Light"),
            DirectionalLight {
                shadows_enabled: true,
                ..default()
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
               SceneRoot(level_assets.demo_level.clone()),
               Level,
           ))
           .add_children(&[player, light, music])
           .id();

    // todo: remove
    // commands.spawn(SceneRoot(level_assets.props.clone()));

    commands.queue(enemy::EnemySpawnCmd {
        transform: Transform::from_xyz(0.0, 0.0, 5.0).with_scale(Vec3::ONE * 1.3),
        parent: Some(level),
    });
    // commands.queue(enemy::EnemySpawnCmd {
    //     pos: Isometry3d::from_translation(vec3(4.0, 0.0, 5.0)),
    //     parent: Some(level),
    // });
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

fn generate_navmesh(
    mut generator: NavmeshGenerator,
    island: Query<&NavMeshHandle3d, With<Island>>,
    navmesh_done: Res<NavmeshDone>,
) {
    if navmesh_done.0 {
        return;
    }
    info!("generating navmesh...");

    let mut count = 0;
    for island in &island {
        count += 1;
        generator.regenerate(
            &island.0,
            NavmeshSettings {
                agent_radius: 0.5,
                ..default()
            },
        );
    }

    info!("regenerated for {count} islands");
}

#[derive(Resource)]
struct NavmeshDone(bool);

fn handle_navmesh_ready(_: On<NavmeshReady>, mut navmesh_done: ResMut<NavmeshDone>) {
    info!("navmesh ready");
    navmesh_done.0 = true;
}

// fn regenrate_navmesh_on_collider_ready(
//     _: On<ColliderConstructorReady>,
//     mut generator: NavmeshGenerator,
//     island: Query<&NavMeshHandle3d, With<Island>>,
// ) {
//     println!("Regenerating navmesh");
//     for island in &island {
//         generator.regenerate(
//             &island.0,
//             NavmeshSettings {
//                 agent_radius: 0.5,
//                 ..default()
//             },
//         );
//     }
// }

#[derive(Resource)]
pub struct NavmeshArchipelagoHolder(pub Entity);
