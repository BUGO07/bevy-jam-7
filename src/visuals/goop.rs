use bevy::{
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::AsBindGroup,
};
use noisy_bevy::NoisyShaderPlugin;

const GOOP_SHADER_PATH: &str = "shaders/goop_vertex.wgsl";

pub type GoopMaterial = ExtendedMaterial<StandardMaterial, GoopMaterialExtention>;

pub fn plugin(app: &mut App) {
    app.add_plugins((MaterialPlugin::<GoopMaterial>::default(), NoisyShaderPlugin));
}

/// Example:
/// ```rust
/// MeshMaterial3d(goop_materials.add(ExtendedMaterial {
///     base: StandardMaterial { ..default() },
///     extension: GoopMaterialExtention::new(2.0, 2.4),
/// })),
/// ```
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct GoopMaterialExtention {
    #[uniform(100)]
    pub noise_frequency: f32,
    #[uniform(100)]
    _webgl2_padding_100: Vec3,

    #[uniform(101)]
    pub extent: f32,
    #[uniform(101)]
    _webgl2_padding_101: Vec3,
}

impl GoopMaterialExtention {
    pub fn new(noise_frequency: f32, extent: f32) -> Self {
        Self {
            noise_frequency,
            _webgl2_padding_100: Vec3::default(),
            extent,
            _webgl2_padding_101: Vec3::default(),
        }
    }
}

impl MaterialExtension for GoopMaterialExtention {
    fn vertex_shader() -> bevy::shader::ShaderRef {
        GOOP_SHADER_PATH.into()
    }
}
