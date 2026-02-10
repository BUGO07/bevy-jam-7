use bevy::prelude::*;

pub struct CheckpointPlugin;

impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        //
    }
}

#[derive(Component)]
pub struct Checkpoint;

#[derive(Component)]
pub struct ActiveCheckpoint;
