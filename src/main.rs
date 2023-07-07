use std::error::Error;

use wgpu::util::DeviceExt;
use winit::event::{Event, WindowEvent};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("生命游戏 wgpu")
        .with_inner_size(winit::dpi::PhysicalSize::<u32>::from((720, 720)))
        .build(&event_loop)?;

    pollster::block_on(run(window, event_loop, (128, 128)))?; //

    Ok(())
}

struct State {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    cycle_render_binding_group: bool,
}

impl State {
    async fn new(window: &winit::window::Window) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN
                | wgpu::Backends::DX12
                | wgpu::Backends::DX11
                | wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window)? };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or("没有合适的适配器")?;

        println!("{:?}", adapter.get_info());

        for flag in adapter.get_downlevel_capabilities().flags.iter() {
            println!("{:?}", flag)
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let swapchain_capabilities = surface.get_capabilities(&adapter);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_capabilities.formats[0],
            width: window.inner_size().width,
            height: window.inner_size().height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Ok(Self {
            _instance: instance,
            surface,
            config,
            _adapter: adapter,
            device,
            queue,
            cycle_render_binding_group: false,
        })
    }

    fn render_binding_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        view1: &wgpu::TextureView,
        view2: &wgpu::TextureView,
        sampler1: &wgpu::Sampler,
        sampler2: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let gen_render_binding_group = |(view, sampler)| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };

        // 渲染部分不参与翻转texture
        // cycle_render_binding_group = !cycle_render_binding_group;
        if self.cycle_render_binding_group {
            gen_render_binding_group((view2, sampler2))
        } else {
            gen_render_binding_group((view1, sampler1))
        }
    }

    fn compute_binding_group(
        &mut self,
        layout: &wgpu::BindGroupLayout,
        view1: &wgpu::TextureView,
        view2: &wgpu::TextureView,

        uniform: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let gen_compute_binding_group = |(read_view, write_view)| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
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
                        resource: uniform.as_entire_binding(),
                    },
                ],
            })
        };

        self.cycle_render_binding_group = !self.cycle_render_binding_group;

        if self.cycle_render_binding_group {
            gen_compute_binding_group((view1, view2))
        } else {
            gen_compute_binding_group((view2, view1))
        }
    }

    fn update_map(
        &self,
        compoute_group: &wgpu::BindGroup,
        pipeline: &wgpu::ComputePipeline,
        map_size: (u32, u32),
    ) {
        let workgroup_count = (
            (map_size.0 as f32 / 16.0).ceil() as u32,
            (map_size.1 as f32 / 16.0).ceil() as u32,
            1,
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

            cpass.set_pipeline(pipeline);
            cpass.set_bind_group(0, compoute_group, &[]);
            cpass.dispatch_workgroups(workgroup_count.0, workgroup_count.1, workgroup_count.2)
        }
        self.queue.submit(Some(encoder.finish()));
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    texcorrd: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
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

async fn run(
    window: winit::window::Window,
    event_loop: winit::event_loop::EventLoop<()>,
    map_size: (u32, u32),
) -> Result<()> {
    let mut state = State::new(&window).await?;

    #[allow(unused_mut)]
    let mut default_map = (0..map_size.0 * map_size.1)
        .map(|_| [0, 0, 0, 0])
        .collect::<Vec<[u8; 4]>>();

    macro_rules! lightup {
        ($x :expr, $y: expr) => {
            default_map[($y * map_size.0 + $x) as usize] = [255u8, 255, 255, 255];
        };
        (all) => {
            for x in 0..map_size.0 {
                for y in 0..map_size.1 {
                    lightup!(x, y);
                }
            }
        };
    }

    for x in 0..10 {
        for y in 0..10 {
            let x = x * 10;
            let y = y * 10;
            lightup!(x + 3, y + 2);
            lightup!(x + 4, y + 3);
            lightup!(x + 2, y + 4);
            lightup!(x + 3, y + 4);
            lightup!(x + 4, y + 4);
        }
    }

    let (texture1, view1, sampler1) = create_map_texture(&state, map_size);
    let (texture2, view2, sampler2) = create_map_texture(&state, map_size);

    for texture in [&texture1, &texture2] {
        state.queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&default_map),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(map_size.0 * std::mem::size_of::<u32>() as u32),
                rows_per_image: Some(map_size.1),
            },
            wgpu::Extent3d {
                width: map_size.0,
                height: map_size.1,
                depth_or_array_layers: 1,
            },
        );
    }

    let render_texture_binding_group_layout =
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
                ],
            });

    let vertices: &[Vertex] = &[
        [-1., 1.0, 0., 0.].into(), // 左上
        [1.0, 1.0, 1., 0.].into(), // 右上
        [1.0, -1., 1., 1.].into(), // 右下
        [-1., -1., 0., 1.].into(), // 左下
    ];
    let vertex_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let indicens: &[u16] = &[0, 1, 2, 0, 2, 3];

    let index_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indicens),
            usage: wgpu::BufferUsages::INDEX,
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
                    bind_group_layouts: &[&render_texture_binding_group_layout],
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

    let compute_binding_group_layout =
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

    let compute_map_size_buffer =
        state
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
                bind_group_layouts: &[&compute_binding_group_layout],
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

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => control_flow.set_exit(),

            WindowEvent::Resized(new_size) => {
                state.config.width = new_size.width;
                state.config.height = new_size.height;
                state.surface.configure(&state.device, &state.config);
            }
            WindowEvent::KeyboardInput {
                input:
                    winit::event::KeyboardInput {
                        state: element_state,
                        virtual_keycode: Some(key_code),
                        ..
                    },
                ..
            } if element_state == winit::event::ElementState::Pressed => match key_code {
                winit::event::VirtualKeyCode::Escape => control_flow.set_exit(),
                winit::event::VirtualKeyCode::N => {
                    let compute_group = state.compute_binding_group(
                        &compute_binding_group_layout,
                        &view1,
                        &view2,
                        &compute_map_size_buffer,
                    );
                    state.update_map(&compute_group, &compute_pipeline, map_size);
                }
                _ => {}
            },
            _ => (),
        },
        Event::RedrawRequested(..) => {
            // 进行渲染

            let render_binding_group = state.render_binding_group(
                &render_texture_binding_group_layout,
                &view1,
                &view2,
                &sampler1,
                &sampler2,
            );

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
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                rpass.set_pipeline(&render_pipeline);
                rpass.set_bind_group(0, &render_binding_group, &[]);
                rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..indicens.len() as u32, 0, 0..1);
            }
            state.queue.submit(Some(encoder.finish()));
            frame.present();

            window.request_redraw();
        }
        _ => {}
    });
}

fn create_map_texture(
    state: &State,
    map_size: (u32, u32),
) -> (wgpu::Texture, wgpu::TextureView, wgpu::Sampler) {
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
    (texture, view, sampler)
}
