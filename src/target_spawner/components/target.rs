use std::time::Duration;

use bevy::{color::palettes::css::WHITE, prelude::*};

use crate::{AudioBuffer, target_spawner::TargetSpawner};
// Marker component for targets
#[derive(Component, Copy, Clone)]
pub struct Target;

#[derive(Message)]
pub struct TargetHit(pub Entity);

#[derive(Event)]
pub struct TargetDestroyed;

#[derive(Message)]
pub struct FireWeapon(pub Transform);

#[derive(Component, DerefMut, Deref, PartialEq, PartialOrd, Clone)]
// Marks when to spawn target
pub struct TargetMarker(pub Duration);

#[derive(Resource)]
pub struct BeatMap {
    pub song: Handle<AudioBuffer>,
    pub hit_targets: Vec<(TargetMarker, Transform)>,
}

impl BeatMap {
    pub fn spawn(
        &self,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands.spawn(AudioPlayer(self.song.clone()));
        let mesh = meshes.add(Sphere::new(1.));
        let mat = materials.add(StandardMaterial::from_color(WHITE));
        for (target, transform) in self.hit_targets.iter().cloned() {
            commands.spawn((
                target,
                transform,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Visibility::Hidden,
            ));
        }
    }

    pub fn save(&mut self, targets: Query<(&TargetMarker, &Transform)>) {
        if targets.is_empty() {
            log::warn!("empty save!");
            return;
        }
        self.hit_targets = targets
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
    }
}

pub fn handle_fire_weapon(
    mut ray_cast: MeshRayCast,
    mut target_hit: MessageWriter<TargetHit>,
    mut fire_weapon: PopulatedMessageReader<FireWeapon>,
    targets: Query<Entity, With<Target>>,
) {
    for FireWeapon(transform) in fire_weapon.read() {
        let ray = Ray3d::new(transform.translation, transform.forward());
        let filter = |entity| targets.contains(entity);
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        let hits = ray_cast.cast_ray(ray, &settings);

        for (entity, _ray_mesh_hit) in hits.iter().take(1) {
            target_hit.write(TargetHit(*entity));
        }
    }
}

pub fn destroy_hit_targets(
    mut targets: PopulatedMessageReader<TargetHit>,
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
) {
    for TargetHit(entity) in targets.read() {
        commands.entity(*entity).remove::<Mesh3d>();
        commands
            .entity(*entity)
            .insert(AudioPlayer::new(asset_server.load("audio/Creams.ogg")));
        _ = commands
            .delayed()
            .secs(2.)
            .get_entity(*entity)
            .and_then(|mut e| {
                e.despawn();
                Ok(())
            });
        commands.trigger(TargetDestroyed);
    }
}

pub fn move_targets(
    spawners: Query<Entity, With<TargetSpawner>>,
    time: ResMut<Time>,
    mut transforms: Query<&mut Transform>,
    child_query: Query<&Children>,
) {
    for spawner in spawners {
        for target in child_query.iter_descendants(spawner) {
            if let Ok(mut transform) = transforms.get_mut(target) {
                transform.translate_around(
                    Vec3::ZERO,
                    Quat::from_euler(EulerRot::XYZ, 0.0, 4.0 * time.delta().as_secs_f32(), 0.0),
                );
            }
        }
    }
}
