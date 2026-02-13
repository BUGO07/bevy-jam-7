// use crate::screens::gameplay::CoefficientCombine;
// use avian3d::prelude::CoefficientCombine;
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::screens::gameplay::character_controller::CharacterControllerBundle;

#[derive(Component, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Player {
    // normalized values (0.0..1.0)
    pub health: f32,
    pub hallucination_severity: f32,
    pub dash_cooldown: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            health: 1.0,
            hallucination_severity: 0.0,
            dash_cooldown: 0.0,
        }
    }
}

impl Player {
    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }
}

pub fn spawn_player(commands: &mut Commands, camera: Entity) -> Entity {
    let player_collider = Collider::capsule(0.4, 1.0);
    commands
        .spawn((
            Name::new("Player"),
            CharacterControllerBundle::new(player_collider.clone()).with_movement(
                1.0,
                0.90,
                10.0,
                35f32.to_radians(),
            ),
            Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            GravityScale(1.5),
            Transform::from_xyz(0.0, 0.9, 2.0),
            Player::default(),
            TransformInterpolation,
            Children::spawn_one((player_collider, Transform::from_xyz(0., 0.9, 0.))),
        ))
        .add_child(camera)
        .id()
}
