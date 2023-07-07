use std::error::Error;

use wgpu::util::DeviceExt;
use winit::event::{Event, WindowEvent};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    // 创建窗口
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("生命游戏 wgpu")
        .with_inner_size(winit::dpi::PhysicalSize::<u32>::from((720, 720)))
        .build(&event_loop)?;

    // wgpu的适配器和设备差创建是异步函数，得用一个异步运行时库
    let mut state = pollster::block_on(State::new(&window))?;

    // 默认地图大小 256 * 256
    let map_size = (512, 512);

    // 默认的地图
    let mut default_map = (0..map_size.0 * map_size.1)
        .map(|_| [0, 0, 0, 0])
        .collect::<Vec<[u8; 4]>>();

    // 用来快速配置地图的宏
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

    // 创建1600个滑翔机
    for x in 0..40 {
        for y in 0..40 {
            let x = x * 10;
            let y = y * 10;
            lightup!(x + 3, y + 2);
            lightup!(x + 4, y + 3);
            lightup!(x + 2, y + 4);
            lightup!(x + 3, y + 4);
            lightup!(x + 4, y + 4);
        }
    }

    // 创建两个纹理
    let textures = [
        Texture::new(&state, map_size),
        Texture::new(&state, map_size),
    ];

    // 都初始化成“初始地图”
    for texture in &textures {
        state.queue.write_texture(
            wgpu::ImageCopyTextureBase {
                texture: &texture.texture,
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

    // 渲染的部分
    let vertices: &[Vertex] = &[
        [-1., 1.0, 0., 0.].into(), // 左上
        [1.0, 1.0, 1., 0.].into(), // 右上
        [1.0, -1., 1., 1.].into(), // 右下
        [-1., -1., 0., 1.].into(), // 左下
    ];
    let indicens: &[u16] = &[0, 1, 2, 0, 2, 3];
    let render = Render::new(&state, vertices, indicens);

    // 更新（计算）的部分
    let mut update = false;
    let compute = Compute::new(&state, map_size);

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
            } => match key_code {
                winit::event::VirtualKeyCode::Escape
                    if element_state == winit::event::ElementState::Pressed =>
                {
                    control_flow.set_exit()
                }
                winit::event::VirtualKeyCode::Space => {
                    update = element_state == winit::event::ElementState::Pressed
                }
                _ => {}
            },
            _ => (),
        },
        Event::RedrawRequested(..) => {
            render.render(&state, &textures);
            if update {
                compute.update(&mut state, &textures)
            }
            window.request_redraw();
        }
        _ => {}
    });
}

struct Texture {
    sampler: wgpu::Sampler,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
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
/// 储存图形部分的状态
struct State {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,

    /// 本质上是创建两个纹理 交替读写来进行更新
    /// 通过来回取反这个量来做到翻转
    cycle_render_binding_group: bool,
}

impl State {
    async fn new(window: &winit::window::Window) -> Result<Self> {
        // 创建实例，展示平面，适配器，设备，命令队列
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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

        // 打印一些调试信息
        println!("{:?}", adapter.get_info());

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

struct Render {
    render_bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indicens_len: u32,
}

impl Render {
    pub fn new(state: &State, vertices: &[Vertex], indicens: &[u16]) -> Self {
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
        }
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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

struct Compute {
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
