use crate::resources::Texture;

use super::{DeviceExt, State};

pub struct Compute {
    compute_bind_group_layout: wgpu::BindGroupLayout,
    compute_pipeline: wgpu::ComputePipeline,
    map_size: (u32, u32),
    map_size_uniform: wgpu::Buffer,
}

impl Compute {
    pub fn new(state: &State, map_size: (u32, u32)) -> Self {
        let compute_bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let map_size_uniform = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[map_size.0 as f32, map_size.1 as f32, 0.0, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let compute_shader_module = state
            .device
            .create_shader_module(wgpu::include_wgsl!("compute.wgsl"));

        let compute_pipeline = {
            let layout = state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                });
            state
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    module: &compute_shader_module,
                    entry_point: "cs_main",
                })
        };
        Self {
            compute_bind_group_layout,
            compute_pipeline,
            map_size,
            map_size_uniform,
        }
    }

    pub fn update(&self, state: &mut State, [texture1, texture2]: &[Texture; 2]) {
        let gen_compute_binding_group = |read_view, write_view| {
            state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(read_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(write_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.map_size_uniform.as_entire_binding(),
                    },
                ],
            })
        };

        state.cycle_render_binding_group = !state.cycle_render_binding_group;

        let compute_bind_group = if state.cycle_render_binding_group {
            gen_compute_binding_group(&texture1.view, &texture2.view)
        } else {
            gen_compute_binding_group(&texture2.view, &texture1.view)
        };

        let workgroup_count = (
            (self.map_size.0 as f32 / 16.0).ceil() as u32,
            (self.map_size.1 as f32 / 16.0).ceil() as u32,
            1,
        );
        let mut encoder = state.device.create_command_encoder(&Default::default());
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &compute_bind_group, &[]);
            cpass.dispatch_workgroups(workgroup_count.0, workgroup_count.1, workgroup_count.2)
        }
        state.queue.submit(Some(encoder.finish()));
    }
}
