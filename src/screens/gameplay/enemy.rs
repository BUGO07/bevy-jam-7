use std::time::Duration;

use avian3d::{math::*, prelude::*};
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_landmass::{PointSampleDistance3d, prelude::*};

use crate::screens::Screen;
use crate::screens::gameplay::LevelAssets;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                enemy_track_nearby_player,
                enemy_move_toward_target,
                update_grounded,
                apply_gravity_system,
                print_desired_velocity.run_if(on_timer(Duration::from_millis(300))),
            )
                .chain()
                .run_if(in_state(Screen::Gameplay)),
        );

        // app.add_systems(
        //     PhysicsSchedule,
        //     enemy_collision.in_set(NarrowPhaseSystems::Last),
        // );
    }
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct Knockback {
    pub velocity: Vec3,
    pub remaining_time: f32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

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
    level_assets: Res<LevelAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    navmesh_ref: Res<super::NavmeshArchipelagoHolder>,
) {
    let enemy_collider = Collider::capsule(0.4, 1.0);
    let mut caster_shape = enemy_collider.clone();
    caster_shape.set_scale(Vec3::ONE * 0.99, 10);

    let mut enemy = c.spawn((
        Name::new("Enemy"),
        Enemy,
        SceneRoot(level_assets.hammerhead.scene.clone()),
        Transform::from_isometry(args.pos),
        Visibility::Inherited,
        RigidBody::Kinematic,
        Agent3dBundle {
            agent: default(),
            archipelago_ref: ArchipelagoRef3d::new(navmesh_ref.0),
            settings: AgentSettings {
                radius: 1.0,
                desired_speed: 3.0,
                max_speed: 4.0,
            },
        },
        AgentTarget3d::None,
        ShapeCaster::new(
            caster_shape,
            Vec3::new(0.0, 0.9, 0.0),
            Quaternion::default(),
            Dir3::NEG_Y,
        )
        .with_max_distance(0.5),
        Children::spawn_one((
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 144, 124))),
            enemy_collider,
            Transform::from_xyz(0.0, 0.9, 0.0),
        )),
    ));

    if let Some(parent) = args.parent {
        enemy.insert(ChildOf(parent));
    }
}

fn print_desired_velocity(query: Query<(Entity, &AgentDesiredVelocity3d, &AgentState)>) {
    for (entity, desired_velocity, state) in query.iter() {
        debug!(
            "entity={:?}, desired_velocity={} {state:?}",
            entity,
            desired_velocity.velocity()
        );
    }
}

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

    let Some(archipelago) = archipelago.iter().next() else { return; };
    let Some((player_entity, player_transform)) = players.iter().next() else { return; };

    for (enemy_transform, mut target) in enemies.iter_mut() {
        let distance = enemy_transform
            .translation
            .distance(player_transform.translation);

        if distance <= DETECTION_RANGE {
            if let Ok(point) = archipelago.sample_point(player_transform.translation, &POINT_SAMPLE_CONFIG) {
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
    mut enemies: Query<
        (&AgentState, &AgentTarget3d, &AgentDesiredVelocity3d, &mut LinearVelocity, &mut Rotation),
        (With<Enemy>, Without<Knockback>),
    >,
) {
    for (state, target, desired_velocity, mut linear_velocity, mut rotation) in enemies.iter_mut() {
        if *state != AgentState::Moving {
            linear_velocity.x = 0.0;
            linear_velocity.z = 0.0;
            continue;
        }
        if !matches!(target, AgentTarget3d::None) {
            linear_velocity.0 = desired_velocity.velocity();
            *rotation = Quat::from_rotation_y(PI / 2.0 - desired_velocity.velocity().xz().to_angle()).into();
        }
    }
}

/// Gravity system for enemies
fn apply_gravity_system(
    time: Res<Time>,
    mut enemies: Query<(&RigidBody, &mut LinearVelocity), (With<Enemy>, Without<Knockback>, Without<Grounded>)>,
) {
    for (rigid_body, mut linear_velocity) in enemies.iter_mut() {
        if rigid_body.is_dynamic() {
            linear_velocity.0 += Vec3::NEG_Y * 9.81 * time.delta_secs();
        }
    }
}

fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation), (With<Enemy>, Without<Knockback>)>,
) {
    for (entity, hits, rotation) in &mut query {
        let is_grounded = hits.iter().any(|hit| {
            (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= 0.1
        });
        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}
