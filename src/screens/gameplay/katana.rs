//! Plays animations from a skinned glTF.

use std::{f32::consts::PI, time::Duration};

use bevy::{animation::RepeatAnimation, light::CascadeShadowConfigBuilder, prelude::*};

use crate::screens::gameplay::{LevelAssets, character_controller::AttackAction, player::Player};

#[derive(Resource)]
pub struct Animations {
    animations: Vec<AnimationNodeIndex>,
    graph_handle: Handle<AnimationGraph>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Katana;

pub fn katana_setup(
    mut commands: Commands,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    level_assets: Res<LevelAssets>,
    player: Single<Entity, With<Camera3d>>,
) {
    // Build the animation graph
    let (graph, node_indices) = AnimationGraph::from_clips([
        level_assets.katana_idle.clone(),
        level_assets.katana_swing.clone(),
    ]);

    // Keep our animation graph in a Resource so that it can be inserted onto
    // the correct entity once the scene actually loads.
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        animations: node_indices,
        graph_handle,
    });

    commands.spawn((
        Name::new("Katana"),
        SceneRoot(level_assets.katana_scene.clone()),
        Transform::from_translation(Vec3::new(-0.1, -0.8, -1.4))
            .with_rotation(Quat::from_rotation_y(0.05))
            .with_scale(Vec3::splat(0.8)),
        ChildOf(player.entity()),
    ));
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the animation.
pub fn poor_setup_for_katana_animations(
    // TODO: system from bevy example, idk how to make it non update
    mut commands: Commands,
    animations: Res<Animations>,
    mut players: Query<(Entity, &mut AnimationPlayer), (Added<AnimationPlayer>, With<Katana>)>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    for (entity, mut player) in &mut players {
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
        *done = true;
    }
}

pub fn katana_animation(
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions), With<Katana>>,
    mut attack_writer: MessageWriter<AttackAction>,
    player_transform: Single<&Transform, With<Player>>,
    animations: Res<Animations>,
    mut non_idle: Local<bool>,
) {
    for (mut player, mut transitions) in &mut animation_players {
        if mouse_input.just_pressed(MouseButton::Left) {
            attack_writer.write(AttackAction::Punch(player_transform.forward()));
            transitions
                .play(
                    &mut player,
                    animations.animations[1],
                    Duration::from_millis(60),
                )
                .set_speed(1.3);
            *non_idle = true;
        }

        if player.all_finished() {
            *non_idle = false;
            transitions
                .play(
                    &mut player,
                    animations.animations[0],
                    Duration::from_millis(250),
                )
                .repeat();
        }
    }
}
