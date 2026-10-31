//! Sky dome and infinite ground. Both are single meshes that follow the camera
//! and are shaded procedurally, so the world looks endless for two draw calls.

use bevy::{
    light::NotShadowCaster,
    math::primitives::{Plane3d, Sphere},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError},
    shader::ShaderRef,
    transform::TransformSystems,
};

use crate::{components::FollowCamera, resources::TrackPath};

/// Shared by sky, ground fade, fog and clear color so the horizon has no seam.
pub const HORIZON: Color = Color::srgb(0.85, 0.18, 0.55);
const ZENITH: Color = Color::srgb(0.03, 0.0, 0.1);
/// Ground and fog fade between these distances from the camera.
pub const FADE_START: f32 = 170.0;
pub const FADE_END: f32 = 720.0;
/// Must stay inside the camera's far plane (1000 by default).
const SKY_RADIUS: f32 = 800.0;

pub struct BackdropPlugin;

impl Plugin for BackdropPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin::<SkyMaterial>::default(), MaterialPlugin::<GroundMaterial>::default()))
            .insert_resource(ClearColor(HORIZON))
            .add_systems(Startup, spawn_backdrop)
            // After the camera moved this frame, before transforms propagate.
            .add_systems(PostUpdate, follow_camera.before(TransformSystems::Propagate));
    }
}

/// Entities that stay centred under/around the camera.
#[derive(Component)]
struct CameraAnchored;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    horizon: LinearRgba,
    #[uniform(0)]
    zenith: LinearRgba,
    #[uniform(0)]
    sun_top: LinearRgba,
    #[uniform(0)]
    sun_bottom: LinearRgba,
    /// xyz: direction to the sun, w: angular radius in radians.
    #[uniform(0)]
    sun: Vec4,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wgsl".into()
    }

    // The camera sits inside the sphere, so draw its inner faces.
    fn specialize(_: &MaterialPipeline, descriptor: &mut RenderPipelineDescriptor, _: &MeshVertexBufferLayoutRef, _: MaterialPipelineKey<Self>) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GroundMaterial {
    #[uniform(0)]
    base: LinearRgba,
    #[uniform(0)]
    line: LinearRgba,
    #[uniform(0)]
    horizon: LinearRgba,
    /// x: cell size, y: line width in pixels, z: fade start, w: fade end.
    #[uniform(0)]
    params: Vec4,
}

impl Material for GroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ground.wgsl".into()
    }
}

fn spawn_backdrop(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut skies: ResMut<Assets<SkyMaterial>>, mut grounds: ResMut<Assets<GroundMaterial>>) {
    let sky = skies.add(SkyMaterial {
        horizon: HORIZON.into(),
        zenith: ZENITH.into(),
        // Values above 1.0 are HDR so bloom makes the sun glow.
        sun_top: LinearRgba::rgb(4.0, 2.6, 0.4),
        sun_bottom: LinearRgba::rgb(3.5, 0.25, 1.2),
        sun: Vec3::new(0.0, 0.035, 1.0).normalize().extend(0.1),
    });
    commands.spawn((Mesh3d(meshes.add(Sphere::new(SKY_RADIUS).mesh().uv(48, 24))), MeshMaterial3d(sky), Transform::default(), CameraAnchored, NotShadowCaster));

    let ground = grounds.add(GroundMaterial {
        base: Color::srgb(0.02, 0.0, 0.05).into(),
        line: LinearRgba::rgb(0.2, 1.6, 2.2),
        horizon: HORIZON.into(),
        params: Vec4::new(20.0, 1.2, FADE_START, FADE_END),
    });
    let size = SKY_RADIUS * 2.0;
    commands.spawn((Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))), MeshMaterial3d(ground), Transform::from_xyz(0.0, TrackPath::GROUND_TOP, 0.0), CameraAnchored, NotShadowCaster));
}

/// Keeps sky and ground centred on the camera horizontally; the ground keeps its height.
fn follow_camera(camera: Query<&Transform, With<FollowCamera>>, mut anchored: Query<&mut Transform, (With<CameraAnchored>, Without<FollowCamera>)>) {
    let Ok(camera) = camera.single() else { return };
    for mut transform in &mut anchored {
        transform.translation.x = camera.translation.x;
        transform.translation.z = camera.translation.z;
    }
}
