use avian3d::{math::*, prelude::*};
use bevy::prelude::*;
use bevy_landmass::{PointSampleDistance3d, prelude::*};

use crate::screens::Screen;

use crate::screens::gameplay::LevelAssets;

pub struct EnemyPlugin;

// FIXME: placeholder to test punching
#[deprecated]
fn enemy_gravity() -> Vec3 {
    Vec3::NEG_Y * 9.81
}

// FIXME: placeholder to test punching
#[deprecated]
const MAX_SLOPE_ANGLE: f32 = 0.1;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                enemy_track_nearby_player,
                enemy_move_toward_target,
                apply_knockback,
                update_grounded,
                apply_gravity,
            )
                .chain()
                .run_if(in_state(Screen::Gameplay)),
        );

        // Run collision handling after collision detection
        app.add_systems(
            PhysicsSchedule,
            enemy_collision.in_set(NarrowPhaseSystems::Last),
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
                radius: 2.0,
                desired_speed: 1.0,
                max_speed: 2.0,
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
            // todo: remove/reuse mesh
            // Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 144, 124))),
            enemy_collider,
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
    mut enemies: Query<
        (&AgentTarget3d, &AgentDesiredVelocity3d, &mut LinearVelocity),
        (With<Enemy>, Without<Knockback>),
    >,
) {
    for (target, desired_velocity, mut linear_velocity) in enemies.iter_mut() {
        if matches!(target, AgentTarget3d::Entity(_)) {
            linear_velocity.0 = desired_velocity.velocity();
        } else {
            linear_velocity.x = 0.0;
            linear_velocity.z = 0.0;
            // Skip zeroing falling
        }
    }
}

fn apply_knockback(
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut LinearVelocity, &mut Knockback), With<Enemy>>,
    time: Res<Time>,
) {
    for (entity, mut linear_velocity, mut knockback) in enemies.iter_mut() {
        knockback.velocity += enemy_gravity() * time.delta_secs();
        linear_velocity.0 = knockback.velocity;
        knockback.remaining_time -= time.delta_secs();

        if knockback.remaining_time <= 0.0 {
            commands.entity(entity).remove::<Knockback>();
        }
    }
}

/// Updates the [`Grounded`] status for character controllers.
fn update_grounded(
    mut commands: Commands,
    mut query: Query<(Entity, &ShapeHits, &Rotation), (With<Enemy>, Without<Knockback>)>,
) {
    for (entity, hits, rotation) in &mut query {
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        let is_grounded = hits.iter().any(|hit| {
            // if let Some(angle) = max_slope_angle {
            let angle = 0.1;
            (rotation * -hit.normal2).angle_between(Vector::Y).abs() <= angle
            // } else {
            //     true
            // }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn apply_gravity(
    time: Res<Time>,
    mut enemies: Query<&mut LinearVelocity, (With<Enemy>, Without<Knockback>, Without<Grounded>)>,
) {
    for mut linear_velocity in enemies.iter_mut() {
        linear_velocity.0 += enemy_gravity() * time.delta_secs();
    }
}

/// Kinematic bodies do not get pushed by collisions by default,
/// so it needs to be done manually.
///
/// This system handles collision response for kinematic character controllers
/// by pushing them along their contact normals by the current penetration depth,
/// and applying velocity corrections in order to snap to slopes, slide along walls,
/// and predict collisions using speculative contacts.
#[allow(clippy::type_complexity)]
fn enemy_collision(
    collisions: Collisions,
    bodies: Query<&RigidBody>,
    collider_rbs: Query<&ColliderOf, Without<Sensor>>,
    mut enemies: Query<(&mut Position, &mut LinearVelocity), With<Enemy>>,
    time: Res<Time>,
) {
    let max_slope_angle = Some(MAX_SLOPE_ANGLE);
    // Iterate through collisions and move the kinematic body to resolve penetration
    for contacts in collisions.iter() {
        // Get the rigid body entities of the colliders (colliders could be children)
        let Ok([&ColliderOf { body: rb1 }, &ColliderOf { body: rb2 }]) =
            collider_rbs.get_many([contacts.collider1, contacts.collider2])
        else {
            continue;
        };

        // Get the body of the character controller and whether it is the first
        // or second entity in the collision.
        let is_first: bool;

        let character_rb: RigidBody;
        let is_other_dynamic: bool;

        let (mut position, mut linear_velocity) = if let Ok(enemy) = enemies.get_mut(rb1) {
            is_first = true;
            character_rb = *bodies.get(rb1).unwrap();
            is_other_dynamic = bodies.get(rb2).is_ok_and(|rb| rb.is_dynamic());
            enemy
        } else if let Ok(character) = enemies.get_mut(rb2) {
            is_first = false;
            character_rb = *bodies.get(rb2).unwrap();
            is_other_dynamic = bodies.get(rb1).is_ok_and(|rb| rb.is_dynamic());
            character
        } else {
            continue;
        };

        // This system only handles collision response for kinematic character controllers.
        if !character_rb.is_kinematic() {
            continue;
        }

        // Iterate through contact manifolds and their contacts.
        // Each contact in a single manifold shares the same contact normal.
        for manifold in contacts.manifolds.iter() {
            let normal = if is_first {
                -manifold.normal
            } else {
                manifold.normal
            };

            let mut deepest_penetration: Scalar = Scalar::MIN;

            // Solve each penetrating contact in the manifold.
            for contact in manifold.points.iter() {
                if contact.penetration > 0.0 {
                    position.0 += normal * contact.penetration;
                }
                deepest_penetration = deepest_penetration.max(contact.penetration);
            }

            // For now, this system only handles velocity corrections for collisions against static geometry.
            if is_other_dynamic {
                continue;
            }

            // Determine if the slope is climbable or if it's too steep to walk on.
            let slope_angle = normal.angle_between(Vector::Y);
            let climbable = max_slope_angle.is_some_and(|angle| slope_angle.abs() <= angle);

            if deepest_penetration > 0.0 {
                // If the slope is climbable, snap the velocity so that the character
                // up and down the surface smoothly.
                if climbable {
                    // Points in the normal's direction in the XZ plane.
                    let normal_direction_xz =
                        normal.reject_from_normalized(Vector::Y).normalize_or_zero();

                    // The movement speed along the direction above.
                    let linear_velocity_xz = linear_velocity.dot(normal_direction_xz);

                    // Snap the Y speed based on the speed at which the character is moving
                    // up or down the slope, and how steep the slope is.
                    //
                    // A 2D visualization of the slope, the contact normal, and the velocity components:
                    //
                    //             ╱
                    //     normal ╱
                    // *         ╱
                    // │   *    ╱   velocity_x
                    // │       * - - - - - -
                    // │           *       | velocity_y
                    // │               *   |
                    // *───────────────────*

                    let max_y_speed = -linear_velocity_xz * slope_angle.tan();
                    linear_velocity.y = linear_velocity.y.max(max_y_speed);
                } else {
                    // The character is intersecting an unclimbable object, like a wall.
                    // We want the character to slide along the surface, similarly to
                    // a collide-and-slide algorithm.

                    // Don't apply an impulse if the character is moving away from the surface.
                    if linear_velocity.dot(normal) > 0.0 {
                        continue;
                    }

                    // Slide along the surface, rejecting the velocity along the contact normal.
                    let impulse = linear_velocity.reject_from_normalized(normal);
                    linear_velocity.0 = impulse;
                }
            } else {
                // The character is not yet intersecting the other object,
                // but the narrow phase detected a speculative collision.
                //
                // We need to push back the part of the velocity
                // that would cause penetration within the next frame.

                let normal_speed = linear_velocity.dot(normal);

                // Don't apply an impulse if the character is moving away from the surface.
                if normal_speed > 0.0 {
                    continue;
                }

                // Compute the impulse to apply.
                let impulse_magnitude =
                    normal_speed - (deepest_penetration / time.delta_secs_f64().adjust_precision());
                let mut impulse = impulse_magnitude * normal;

                // Apply the impulse differently depending on the slope angle.
                if climbable {
                    // Avoid sliding down slopes.
                    linear_velocity.y -= impulse.y.min(0.0);
                } else {
                    // Avoid climbing up walls.
                    impulse.y = impulse.y.max(0.0);
                    linear_velocity.0 -= impulse;
                }
            }
        }
    }
}
