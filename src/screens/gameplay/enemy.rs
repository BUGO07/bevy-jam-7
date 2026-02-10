use avian3d::prelude::*;
use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, _app: &mut App) {
        //
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
) {
    let mut enemy = c.spawn((
        Name::new("Enemy"),
        Enemy,
        Transform::from_isometry(args.pos),
        Visibility::Inherited,
        RigidBody::Static,
        Children::spawn_one((
            Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb_u8(255, 144, 124))),
            Collider::capsule(0.4, 1.0),
        )),
    ));

    if let Some(parent) = args.parent {
        enemy.insert(ChildOf(parent));
    }
}
