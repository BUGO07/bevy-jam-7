use avian3d::prelude::LinearVelocity;
use bevy::{camera::visibility::NoFrustumCulling, prelude::*, scene::SceneInstanceReady};

use crate::{menus::Menu, screens::gameplay::Player};

pub struct CheckpointPlugin;

impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(move_player_to_checkpoint);
        app.add_systems(OnEnter(Menu::None), respawn_at_checkpoint);
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Checkpoint;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ActiveCheckpoint;

fn respawn_at_checkpoint(
    player: Single<(&mut Transform, &mut LinearVelocity, &mut Player), Without<ActiveCheckpoint>>,
    active_checkpoint: Single<&Transform, With<ActiveCheckpoint>>,
) {
    let (mut transform, mut linear_velocity, mut player) = player.into_inner();

    if player.is_alive() {
        return;
    }

    *player = Default::default();
    *linear_velocity = Default::default();

    // spawn above the checkpoint so player doesn't fall through the floor
    transform.translation = active_checkpoint.translation + Vec3::Y;
}

fn move_player_to_checkpoint(
    _: On<SceneInstanceReady>,
    mut commands: Commands,
    mut player: Single<&mut Transform, With<Player>>,
    mesh3d: Query<Entity, With<Mesh3d>>,
    active_checkpoint: Single<&Transform, (With<ActiveCheckpoint>, Without<Player>)>,
) {
    for entity in mesh3d.iter() {
        commands.entity(entity).insert(NoFrustumCulling);
    }
    player.translation = active_checkpoint.translation + Vec3::Y;
}
