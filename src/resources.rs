use crate::State;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
    texcorrd: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
            0=> Float32x2,
            1=> Float32x2,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 2 * 2]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl From<[[f32; 2]; 2]> for Vertex {
    fn from([position, texcorrd]: [[f32; 2]; 2]) -> Self {
        Self { position, texcorrd }
    }
}

impl From<[f32; 4]> for Vertex {
    fn from([position_x, position_y, uvx, uvy]: [f32; 4]) -> Self {
        [[position_x, position_y], [uvx, uvy]].into()
    }
}
pub struct Texture {
    pub sampler: wgpu::Sampler,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl Texture {
    pub fn new(state: &State, map_size: (u32, u32)) -> Texture {
        let texture_size = wgpu::Extent3d {
            width: map_size.0,
            height: map_size.1,
            depth_or_array_layers: 1,
        };

        let texture = state.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Texture {
            sampler,
            texture,
            view,
        }
    }
}
