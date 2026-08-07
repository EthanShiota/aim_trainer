// Timing Ring
// Ring which always faces the camera and shrinks over time

use crate::{GameState, fps_camera::FPSCamera};
use bevy::{color::palettes::css::WHITE_SMOKE, prelude::*, render::render_resource::AsBindGroup};

pub struct TimingRingPlugin;

impl Plugin for TimingRingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<RingMaterial>::default())
            .init_resource::<TimingRingResource<RingMaterial>>()
            .add_systems(Update, tick.run_if(in_state(GameState::Playing)))
            .world_mut()
            .register_component_hooks::<TimingRing>()
            .on_add(|mut world, ctx| {
                let TimingRingResource { mesh, material, .. } =
                    world.resource::<TimingRingResource<RingMaterial>>().clone();
                let mut commands = world.commands();
                commands.entity(ctx.entity).apply_scene(bsn! {
                    Mesh3d(mesh)
                    MeshMaterial3d::<RingMaterial>(material)
                });
            });
    }
}

#[derive(Resource, Clone)]
pub struct TimingRingResource<M: Material> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<M>,
    pub easing: EasingCurve<f32>,
}
impl FromWorld for TimingRingResource<RingMaterial> {
    fn from_world(world: &mut World) -> Self {
        let mesh: Handle<Mesh> = world.add_asset(Torus::new(1., 1.5));
        let material: Handle<RingMaterial> = world.add_asset(RingMaterial {
            color: WHITE_SMOKE.into(),
        });

        Self {
            mesh,
            material,
            easing: EasingCurve::new(0., 1., EaseFunction::ExponentialIn),
        }
    }
}

#[derive(Component)]
/// Shrinks over timer time to end scale
pub struct TimingRing {
    pub lifetime: Timer,
    pub end_scale: f32,
}

fn tick(
    mut q_ring: Query<(&mut TimingRing, &mut Transform)>,
    fps_camera_transform: Single<&Transform, (With<FPSCamera>, Without<TimingRing>)>,
    timing_ring_res: Res<TimingRingResource<RingMaterial>>,
    time: Res<Time<Virtual>>,
) {
    for (mut ring, mut transform) in q_ring.iter_mut() {
        ring.lifetime.tick(time.delta());
        let i = ring.lifetime.remaining_secs() / ring.lifetime.duration().as_secs_f32();
        let i = timing_ring_res.easing.sample(i).unwrap();

        transform.scale = Vec3::splat((1.).lerp(ring.end_scale, i));

        let vector_to_cam = fps_camera_transform.translation - transform.translation;
        transform.align(Dir3::Z, vector_to_cam, Vec3::ZERO, Vec3::ZERO);
    }
}

#[derive(Asset, TypePath, Clone, AsBindGroup)]
pub struct RingMaterial {
    #[uniform(100)]
    color: LinearRgba,
}

impl Material for RingMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        // bevy::shader::ShaderRef::from("shaders/ring_material.wgsl")

        "shaders/ring_material.wgsl".into()
    }

    fn opaque_render_method(&self) -> bevy::material::OpaqueRendererMethod {
        bevy::material::OpaqueRendererMethod::Forward
    }
}
