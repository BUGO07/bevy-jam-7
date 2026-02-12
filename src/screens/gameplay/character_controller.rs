use avian3d::prelude::*;
use bevy::{ecs::query::Has, input::mouse::MouseMotion, prelude::*, window::PrimaryWindow};

use super::enemy::{Enemy, Knockback};
use crate::{
    PausableSystems,
    audio::sound_effect,
    screens::gameplay::{
        Level, LevelAssets, Player,
        checkpoints::{ActiveCheckpoint, Checkpoint},
    },
};

use crate::screens::Screen;
use crate::screens::gameplay::spawn_level;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<MovementAction>()
            .add_message::<AttackAction>()
            .add_systems(
                OnEnter(Screen::Gameplay),
                spawn_something_punchable.after(spawn_level),
            )
            .add_systems(
                Update,
                (
                    kbm_input,
                    gamepad_input,
                    update_grounded,
                    movement,
                    apply_movement_damping,
                    attack,
                    // ray_cast,
                )
                    .chain()
                    .in_set(PausableSystems),
            );
    }
}

#[derive(Message)]
pub enum MovementAction {
    Move(Vec2, f32),
    Look(Vec2),
    Dash(Vec2),
    Jump,
}

#[derive(Message)]
pub enum AttackAction {
    Punch(Dir3),
}

#[derive(Component)]
pub struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component)]
pub struct MovementAcceleration(f32);

#[derive(Component)]
pub struct MovementDampingFactor(f32);

#[derive(Component)]
pub struct JumpImpulse(f32);

#[derive(Component)]
pub struct MaxSlopeAngle(f32);

#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    body: RigidBody,
    ground_caster: ShapeCaster,
    locked_axes: LockedAxes,
    movement: MovementBundle,
}

#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,
    damping: MovementDampingFactor,
    jump_impulse: JumpImpulse,
    max_slope_angle: MaxSlopeAngle,
}

impl MovementBundle {
    pub const fn new(
        acceleration: f32,
        damping: f32,
        jump_impulse: f32,
        max_slope_angle: f32,
    ) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),
            damping: MovementDampingFactor(damping),
            jump_impulse: JumpImpulse(jump_impulse),
            max_slope_angle: MaxSlopeAngle(max_slope_angle),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 0.9, 7.0, std::f32::consts::PI * 0.45)
    }
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider) -> Self {
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vec3::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController,
            body: RigidBody::Dynamic,
            ground_caster: ShapeCaster::new(
                caster_shape,
                Vec3::Y * 0.9,
                Quat::default(),
                Dir3::NEG_Y,
            )
            .with_max_distance(0.2)
            .with_max_hits(10),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            movement: MovementBundle::default(),
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: f32,
        damping: f32,
        jump_impulse: f32,
        max_slope_angle: f32,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, damping, jump_impulse, max_slope_angle);
        self
    }
}

fn kbm_input(
    mut movement_writer: MessageWriter<MovementAction>,
    mut attack_writer: MessageWriter<AttackAction>,
    mut mouse_input: MessageReader<MouseMotion>,
    player: Single<(&mut Player, &Transform)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let (mut player, transform) = player.into_inner();
    let up = keyboard_input.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
    let down = keyboard_input.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
    let left = keyboard_input.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
    let right = keyboard_input.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);
    let dash = keyboard_input.just_pressed(KeyCode::ShiftLeft);
    let punch = keyboard_input.just_pressed(KeyCode::KeyV);
    let damage = keyboard_input.just_pressed(KeyCode::KeyH);

    if damage {
        player.health = (player.health - 0.25).max(0.0);
    }

    let horizontal = right as i8 - left as i8;
    let vertical = up as i8 - down as i8;
    let direction = Vec2::new(horizontal as f32, vertical as f32).clamp_length_max(1.0);

    if direction != Vec2::ZERO {
        if dash && player.dash_cooldown <= 0.0 {
            player.dash_cooldown = 1.5;
            movement_writer.write(MovementAction::Dash(direction * 500.0));
        } else {
            movement_writer.write(MovementAction::Move(direction, 1.0));
        }
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        movement_writer.write(MovementAction::Jump);
    }

    if punch {
        attack_writer.write(AttackAction::Punch(transform.forward()));
    }

    for motion in mouse_input.read() {
        movement_writer.write(MovementAction::Look(motion.delta));
    }
}

fn gamepad_input(
    mut movement_writer: MessageWriter<MovementAction>,
    mut player: Single<&mut Player>,
    gamepads: Query<&Gamepad>,
) {
    for gamepad in gamepads.iter() {
        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::LeftStickX),
            gamepad.get(GamepadAxis::LeftStickY),
        ) {
            let direction = Vec2::new(x, y).clamp_length_max(1.0);
            let dash = gamepad.just_pressed(GamepadButton::East);
            if dash && player.dash_cooldown <= 0.0 {
                player.dash_cooldown = 1.5;
                movement_writer.write(MovementAction::Dash(direction * 200.0));
            } else {
                movement_writer.write(MovementAction::Move(
                    direction,
                    gamepad.get(GamepadButton::RightTrigger2).unwrap_or(0.0) * 0.5 + 1.0,
                ));
            }
        }

        if let (Some(x), Some(y)) = (
            gamepad.get(GamepadAxis::RightStickX),
            gamepad.get(GamepadAxis::RightStickY),
        ) {
            movement_writer.write(MovementAction::Look(
                Vec2::new(x, -y).clamp_length_max(1.0) * 10.0,
            ));
        }

        if gamepad.just_pressed(GamepadButton::South) {
            movement_writer.write(MovementAction::Jump);
        }
    }
}

fn update_grounded(
    mut commands: Commands,
    mut query: Query<
        (Entity, &ShapeHits, &Rotation, Option<&MaxSlopeAngle>),
        With<CharacterController>,
    >,
    checkpoints: Query<Entity, With<Checkpoint>>,
    active_checkpoint: Single<Entity, With<ActiveCheckpoint>>,
) {
    for (entity, hits, rotation, max_slope_angle) in &mut query {
        let is_grounded = hits.iter().any(|hit| {
            if let Ok(checkpoint) = checkpoints.get(hit.entity) {
                commands
                    .entity(*active_checkpoint)
                    .remove::<ActiveCheckpoint>();
                commands.entity(checkpoint).insert(ActiveCheckpoint);
            }
            if let Some(angle) = max_slope_angle {
                (rotation * -hit.normal2).angle_between(Vec3::Y).abs() <= angle.0
            } else {
                true
            }
        });

        if is_grounded {
            commands.entity(entity).insert(Grounded);
        } else {
            commands.entity(entity).remove::<Grounded>();
        }
    }
}

fn movement(
    mut commands: Commands,
    mut movement_reader: MessageReader<MovementAction>,
    mut controllers: Query<(
        &MovementAcceleration,
        &JumpImpulse,
        &mut LinearVelocity,
        &mut Transform,
        Has<Grounded>,
    )>,
    mut camera: Single<&mut Transform, (With<Camera3d>, Without<MovementAcceleration>)>,
    mut player: Single<&mut Player>,
    level: Single<Entity, With<Level>>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    level_assets: Res<LevelAssets>,
    mut sound_cooldown: Local<f32>,
) {
    for event in movement_reader.read() {
        for (
            movement_acceleration,
            jump_impulse,
            mut linear_velocity,
            mut transform,
            is_grounded,
        ) in &mut controllers
        {
            match event {
                MovementAction::Move(direction, speed_multiplier) => {
                    let local_z = transform.rotation * Vec3::Z;
                    let forward = -Vec3::new(local_z.x, 0.0, local_z.z).normalize_or_zero();
                    let right = Vec3::new(local_z.z, 0.0, -local_z.x).normalize_or_zero();
                    let movement_direction = forward * direction.y + right * direction.x;
                    linear_velocity.0 +=
                        movement_direction * movement_acceleration.0 * speed_multiplier;

                    let length =
                        movement_direction.length() * movement_acceleration.0 * speed_multiplier;
                    if is_grounded && length > 0.05 && *sound_cooldown <= 0.0 {
                        commands
                            .entity(*level)
                            .with_child(sound_effect(level_assets.step1.clone()));
                        *sound_cooldown = 0.2 / length;
                    }
                }
                // SAME AS MOVE BUT WITH EXTRA Y VELOCITY
                MovementAction::Dash(direction) => {
                    let local_z = transform.rotation * Vec3::Z;
                    let forward = -Vec3::new(local_z.x, 0.0, local_z.z).normalize_or_zero();
                    let right = Vec3::new(local_z.z, 0.0, -local_z.x).normalize_or_zero();
                    linear_velocity.0.y = 0.0; // reset y velocity so dash is consistent even if you're falling
                    let movement_direction =
                        forward * direction.y + right * direction.x + Vec3::Y * 10.0;
                    linear_velocity.0 += movement_direction * movement_acceleration.0 * 0.1;
                    commands
                        .entity(*level)
                        .with_child(sound_effect(level_assets.whoosh1.clone()));
                }
                MovementAction::Look(direction) => {
                    let (mut yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
                    let (_, mut pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
                    let window_scale = window.height().max(window.width());

                    pitch -= (1.2 * direction.y * window_scale / 10_000.0).to_radians();
                    yaw -= (1.2 * direction.x * window_scale / 10_000.0).to_radians();

                    pitch = pitch.clamp(-1.54, 1.54);

                    transform.rotation = Quat::from_rotation_y(yaw);
                    camera.rotation = Quat::from_rotation_x(pitch);
                }
                MovementAction::Jump => {
                    if is_grounded {
                        linear_velocity.y = jump_impulse.0;
                    }
                }
            }
        }
    }
    player.dash_cooldown -= time.delta_secs();
    *sound_cooldown -= time.delta_secs();
}

fn attack(
    mut attack_reader: MessageReader<AttackAction>,
    mut commands: Commands,
    player_transform: Single<&Transform, With<Player>>,
    mut punchables: Query<
        (&GlobalTransform, Forces),
        (With<Collider>, Without<Player>, Without<Enemy>),
    >,
    mut enemies: Query<(Entity, &GlobalTransform), With<Enemy>>,
    level_assets: Res<LevelAssets>,
    level: Single<Entity, With<Level>>,
) {
    fn punch_impulse(
        target_transform: &GlobalTransform,
        player_transform: &Transform,
        punch_forward: &Dir3,
    ) -> Option<Vec3> {
        const PUNCH_RANGE: f32 = 2.0;
        const PUNCH_FORCE: f32 = 15.0;
        // Right now we only care it's in player forward direction
        // Consider other ways(maybe ray-cast?) to check if `Punchable` is there
        const MIN_DOT_PRODUCT: f32 = 0.1;

        let target_pos = target_transform.translation();

        let to_object_from_player = target_pos - (*player_transform).translation;
        let distance = to_object_from_player.length();

        let push_direction = (to_object_from_player
            + Vec3 {
                y: 0.2,
                ..default()
            })
        .normalize();
        let dot_product = punch_forward.dot(push_direction);
        if distance < PUNCH_RANGE && dot_product > MIN_DOT_PRODUCT {
            Some(push_direction * PUNCH_FORCE)
        } else {
            None
        }
    }

    for event in attack_reader.read() {
        match event {
            AttackAction::Punch(punch_forward) => {
                for (transform, mut forces) in punchables.iter_mut() {
                    if let Some(impulse) =
                        punch_impulse(transform, *player_transform, punch_forward)
                    {
                        forces.apply_linear_impulse(impulse);
                    }
                }

                for (entity, transform) in enemies.iter_mut() {
                    if let Some(impulse) =
                        punch_impulse(transform, *player_transform, punch_forward)
                    {
                        commands.entity(entity).insert(Knockback {
                            velocity: impulse,
                            remaining_time: 0.3,
                        });
                    }
                }

                // TODO: punch sound effect
                commands
                    .entity(*level)
                    .with_child(sound_effect(level_assets.whoosh1.clone()));
            }
        }
    }
}

fn spawn_something_punchable(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    level: Single<Entity, With<Level>>,
) {
    let cube = commands
        .spawn((
            Name::new("PunchableCube"),
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            Transform::from_xyz(2.0, 1.5, -3.0),
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 1.0),
            Mass(5.0),
        ))
        .id();

    commands.entity(*level).add_child(cube);
}

#[allow(unused)]
fn ray_cast(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    children: Query<(Entity, &ChildOf)>, // child because material is a child of the object in blender or smth
    player: Single<(Entity, &Transform), With<Player>>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
    query: SpatialQuery,
    time: Res<Time>,
    mut last_time: Local<Option<f32>>,
) {
    let last_time = last_time.get_or_insert(time.elapsed_secs() - 0.5);
    let camera = camera.into_inner();
    let (player, transform) = player.into_inner();
    if let Some(hit) = query.cast_ray(
        camera.translation() + transform.forward() * 0.4,
        camera.forward(),
        5.0,
        true,
        &SpatialQueryFilter::from_excluded_entities([player]),
    ) {
        if time.elapsed_secs() - *last_time >= 0.5
            && let Some((entity, _)) = children.iter().find(|(_, c)| c.0 == hit.entity)
        {
            commands
                .entity(entity)
                .insert(MeshMaterial3d(materials.add(Color::srgb(
                    rand::random(),
                    rand::random(),
                    rand::random(),
                ))));
            *last_time = time.elapsed_secs();
        }
        // info!(
        //     "Hit entity {:?} at distance of {:?} with normal {:?}",
        //     hit.entity, hit.distance, hit.normal
        // );
    } else {
        *last_time = time.elapsed_secs() - 0.5;
    }
}

fn apply_movement_damping(mut query: Query<(&MovementDampingFactor, &mut LinearVelocity)>) {
    for (damping_factor, mut linear_velocity) in &mut query {
        linear_velocity.x *= damping_factor.0;
        linear_velocity.z *= damping_factor.0;
    }
}
