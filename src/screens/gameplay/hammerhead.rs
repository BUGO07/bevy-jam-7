use std::{f32::consts::PI, time::Duration};

use bevy::{animation::RepeatAnimation, prelude::*};

use crate::screens::{Screen, gameplay::LevelAssets};

const HAMMERHEAD: &str = "models/hammerhead.glb";

pub fn hammerhead(app: &mut App) {
    app.add_systems(OnEnter(Screen::Gameplay), setup)
        .add_systems(
            Update,
            (setup_scene_once_loaded, keyboard_control).run_if(in_state(Screen::Gameplay)),
        );
}

#[derive(Asset, Clone, Reflect)]
pub struct HammerheadAssets {
    #[dependency]
    pub scene: Handle<Scene>,

    #[dependency]
    pub animations: Vec<Handle<AnimationClip>>,
}

impl HammerheadAssets {
    pub fn load(assets: &AssetServer) -> Self {
        Self {
            scene: assets.load(GltfAssetLabel::Scene(0).from_asset(HAMMERHEAD)),
            animations: vec![
                // asset_server.load(GltfAssetLabel::Animation(2).from_asset(HAMMERHEAD)),
                assets.load(GltfAssetLabel::Animation(1).from_asset(HAMMERHEAD)),
                assets.load(GltfAssetLabel::Animation(0).from_asset(HAMMERHEAD)),
            ],
        }
    }
}

#[derive(Resource)]
struct HammerheadAnimations {
    animations: Vec<AnimationNodeIndex>,
    graph_handle: Handle<AnimationGraph>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    level_assets: Res<LevelAssets>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(HAMMERHEAD)),
    ));

    // Build the animation graph
    let (graph, node_indices) =
        AnimationGraph::from_clips(level_assets.hammerhead.animations.clone());

    // Keep our animation graph in a Resource so that it can be inserted onto
    // the correct entity once the scene actually loads.
    let graph_handle = graphs.add(graph);
    commands.insert_resource(HammerheadAnimations {
        animations: node_indices,
        graph_handle,
    });

    // Instructions

    commands.spawn((
        Text::new(concat!(
            "space: play / pause\n",
            "up / down: playback speed\n",
            "left / right: seek\n",
            "1-3: play N times\n",
            "L: loop forever\n",
            "return: change animation\n",
        )),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the animation.
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<HammerheadAnimations>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        info!("setting up scene once loaded...");

        let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        transitions
            .play(&mut player, animations.animations[0], Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph_handle.clone()))
            .insert(transitions);
    }
}

fn keyboard_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<HammerheadAnimations>,
    mut current_animation: Local<usize>,
) {
    for (mut player, mut transitions) in &mut animation_players {
        let Some((&playing_animation_index, _)) = player.playing_animations().next() else {
            continue;
        };

        if keyboard_input.just_pressed(KeyCode::Space) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            if playing_animation.is_paused() {
                playing_animation.resume();
            } else {
                playing_animation.pause();
            }
        }

        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let speed = playing_animation.speed();
            playing_animation.set_speed(speed * 1.2);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let speed = playing_animation.speed();
            playing_animation.set_speed(speed * 0.8);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let elapsed = playing_animation.seek_time();
            playing_animation.seek_to(elapsed - 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let elapsed = playing_animation.seek_time();
            playing_animation.seek_to(elapsed + 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Enter) {
            *current_animation = (*current_animation + 1) % animations.animations.len();

            transitions
                .play(
                    &mut player,
                    animations.animations[*current_animation],
                    Duration::from_millis(250),
                )
                .repeat();
        }

        if keyboard_input.just_pressed(KeyCode::Digit1) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(1))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit2) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(2))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit3) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(3))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::KeyL) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation.set_repeat(RepeatAnimation::Forever);
        }
    }
}
