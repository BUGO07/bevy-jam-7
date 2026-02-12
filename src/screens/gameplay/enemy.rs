use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_landmass::{PointSampleDistance3d, prelude::*};

use crate::screens::Screen;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (enemy_track_nearby_player, enemy_move_toward_target)
                .chain()
                .run_if(in_state(Screen::Gameplay)),
        );

        // #[cfg(feature = "dev")]
        // {
        //     app.add_systems(
        //         Update,
        //         print_desired_velocity.run_if(bevy::input::common_conditions::input_toggle_active(
        //             false,
        //             crate::dev_tools::TOGGLE_KEY,
        //         )),
        //     );
        // }
    }
}

#[derive(Component)]
pub struct Enemy;

pub struct EnemySpawnCmd {
    pub pos: Isometry3d,
    pub parent: Option<Entity>,
}

impl Command for EnemySpawnCmd {
    fn apply(self, world: &mut World) {
        world.run_system_cached_with(spawn_enemy, self).unwrap();
    }
}

fn spawn_enemy(
    In(args): In<EnemySpawnCmd>,
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    navmesh_ref: Res<super::NavmeshArchipelagoHolder>,
) {
    let mut enemy = c.spawn((
        Name::new("Enemy"),
        Enemy,
        Transform::from_isometry(args.pos),
        Visibility::Inherited,
        RigidBody::Kinematic,
        Agent3dBundle {
            agent: default(),
            archipelago_ref: ArchipelagoRef3d::new(navmesh_ref.0),
            settings: AgentSettings {
                radius: 2.0,
                desired_speed: 1.0,
                max_speed: 2.0,
            },
        },
        AgentTarget3d::None,
        Children::spawn_one((
            Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 144, 124))),
            Collider::capsule(0.4, 1.0),
            Transform::from_xyz(0.0, 0.9, 0.0),
        )),
    ));

    if let Some(parent) = args.parent {
        enemy.insert(ChildOf(parent));
    }
}

// fn print_desired_velocity(query: Query<(Entity, &AgentDesiredVelocity3d, &AgentState)>) {
//     for (entity, desired_velocity, s) in query.iter() {
//         println!(
//             "entity={:?}, desired_velocity={} {s:?}",
//             entity,
//             desired_velocity.velocity()
//         );
//     }
// }

fn enemy_track_nearby_player(
    mut enemies: Query<(&Transform, &mut AgentTarget3d), With<Enemy>>,
    players: Query<(Entity, &Transform), With<super::Player>>,
    archipelago: Query<&Archipelago3d>,
) {
    const DETECTION_RANGE: f32 = 5.0;

    const POINT_SAMPLE_CONFIG: PointSampleDistance3d = PointSampleDistance3d {
        animation_link_max_vertical_distance: 50.,
        distance_above: 50.,
        distance_below: 50.,
        horizontal_distance: 50.,
        vertical_preference_ratio: 1.0,
    };

    let Some(archipelago) = archipelago.iter().next() else {
        return;
    };

    let Some((player_entity, player_transform)) = players.iter().next() else {
        return;
    };

    for (enemy_transform, mut target) in enemies.iter_mut() {
        let distance = enemy_transform
            .translation
            .distance(player_transform.translation);

        if distance <= DETECTION_RANGE {
            if let Ok(point) =
                archipelago.sample_point(player_transform.translation, &POINT_SAMPLE_CONFIG)
            {
                *target = AgentTarget3d::Point(point.point());
            } else {
                *target = AgentTarget3d::Entity(player_entity);
            }
        } else {
            *target = AgentTarget3d::None;
        }
    }
}

fn enemy_move_toward_target(
    mut enemies: Query<(&AgentTarget3d, &AgentDesiredVelocity3d, &mut LinearVelocity), With<Enemy>>,
) {
    for (_target, desired_velocity, mut linear_velocity) in enemies.iter_mut() {
        linear_velocity.0 = desired_velocity.velocity();
    }
}
