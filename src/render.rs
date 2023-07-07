use crate::{
    resources::{Texture, Vertex},
    DeviceExt, State,
};

pub struct Render {
    render_bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    camera_uniform: wgpu::Buffer,
    // offset : vec2f
    indicens_len: u32,
}

impl Render {
    pub fn new(
        state: &State,
        vertices: &[Vertex],
        indicens: &[u16],
        view_proj: glam::Mat4,
    ) -> Self {
        let render_bind_group_layout =
            state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::all(),
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::all(),
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::all(),
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let vertex_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indicens),
                usage: wgpu::BufferUsages::INDEX,
            });

        let camera_uniform = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&view_proj.to_cols_array_2d()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let render_shader_module = state
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline = {
            let pipeline_layout =
                state
                    .device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: None,
                        bind_group_layouts: &[&render_bind_group_layout],
                        push_constant_ranges: &[],
                    });

            state
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &render_shader_module,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &render_shader_module,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: state.config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent::REPLACE,
                                alpha: wgpu::BlendComponent::REPLACE,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview: None,
                })
        };
        Self {
            render_bind_group_layout,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            indicens_len: indicens.len() as u32,
            camera_uniform,
        }
    }

    pub fn update_camera_uniform(&self, state: &State, view_proj: glam::Mat4) {
        state.queue.write_buffer(
            &self.camera_uniform,
            0,
            bytemuck::cast_slice(&view_proj.to_cols_array()),
        );
    }

    pub fn render(&self, state: &State, [texture1, texture2]: &[Texture; 2]) {
        let gen_render_binding_group = |texture: &Texture| {
            state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.render_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(
                            self.camera_uniform.as_entire_buffer_binding(),
                        ),
                    },
                ],
            })
        };

        // 渲染部分不参与翻转texture
        // cycle_render_binding_group = !cycle_render_binding_group;
        let render_bind_group = if state.cycle_render_binding_group {
            gen_render_binding_group(texture2)
        } else {
            gen_render_binding_group(texture1)
        };

        let frame = state.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.4,
                            g: 0.4,
                            b: 0.4,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &render_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..self.indicens_len, 0, 0..1);
        }
        state.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
