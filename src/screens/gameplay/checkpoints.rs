use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use crate::{menus::Menu, screens::gameplay::Player};

pub struct CheckpointPlugin;

impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
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
    transform.translation = active_checkpoint.translation;
    transform.translation.y += 1.0; // spawn above the checkpoint so player doesn't fall through the floor
}
