mod compute;
mod mvp;
mod render;
mod resources;

use std::{cell::OnceCell, error::Error, time::Instant};

use resources::{Texture, Vertex};
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    keyboard::KeyCode,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    // 创建窗口
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new()
        .with_title("生命游戏 wgpu")
        .with_inner_size(winit::dpi::PhysicalSize::<u32>::from((720, 720)))
        .build(&event_loop)?;

    // wgpu的适配器和设备差创建是异步函数，得用一个异步运行时库
    let mut state = pollster::block_on(State::new(&window))?;

    // 地图大小
    let map_size = (2048, 2048);

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

    // 创建10,000个滑翔机
    for x in 0..100 {
        for y in 0..100 {
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

    // 投影& 透视

    // 速度  （移动）：1.0
    // 灵敏度（鼠标）：8.0
    let mut camera = mvp::Camera::new([0.0, 0.0, 1.0], 1.0, 8.0);
    let mut projection = mvp::Projection::new(
        window.inner_size().width,
        window.inner_size().height,
        30.,
        0.1,
        100.0,
    );
    let mut camera_controler = mvp::CameraController::new();

    // 渲染的部分
    let vertices: &[Vertex] = &[
        [-1., 1.0, 0., 0.].into(), // 左上
        [1.0, 1.0, 1., 0.].into(), // 右上
        [1.0, -1., 1., 1.].into(), // 右下
        [-1., -1., 0., 1.].into(), // 左下
    ];
    let indicens: &[u16] = &[0, 1, 2, 0, 2, 3];
    let render = render::Render::new(
        &state,
        vertices,
        indicens,
        projection.calc_matrix() * camera.calc_matrix(),
    );

    // 更新（计算）的部分
    let mut update = false;
    let compute = compute::Compute::new(&state, map_size);

    let mut last_frame: OnceCell<Instant> = OnceCell::new();

    Ok(event_loop.run(move |event, loop_target| {
        last_frame.get_or_init(Instant::now);
        let dt = last_frame.get().unwrap().elapsed();
        if let Some(instant) = last_frame.get_mut() {
            *instant = Instant::now()
        };
        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => loop_target.exit(),

                WindowEvent::Resized(new_size) if new_size.width > 0 && new_size.height > 0 => {
                    state.config.width = new_size.width;
                    state.config.height = new_size.height;
                    state.surface.configure(&state.device, &state.config);
                    projection.resize(new_size.width, new_size.height);
                }
                WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            state: element_state,
                            physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                            // virtual_keycode: Some(key_code),
                            ..
                        },
                    ..
                } if !camera_controler.process_keyboard(key_code, element_state) => {
                    match key_code {
                        KeyCode::Escape if element_state == winit::event::ElementState::Pressed => {
                            loop_target.exit()
                        }
                        KeyCode::KeyN if element_state == winit::event::ElementState::Pressed => {
                            compute.update(&mut state, &textures)
                        }
                        KeyCode::Space => {
                            update = element_state == winit::event::ElementState::Pressed
                        }
                        _ => {}
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => camera_controler.process_wheel(delta, dt),
                WindowEvent::RedrawRequested => {
                    camera_controler.update_camera(&mut camera, dt);

                    render.update_camera_uniform(
                        &state,
                        projection.calc_matrix() * camera.calc_matrix(),
                    );
                    render.render(&state, &textures);
                    if update {
                        compute.update(&mut state, &textures)
                    }
                    window.request_redraw();
                }
                _ => (),
            },
            _ => (),
        }
    })?)
}

/// 储存图形部分的状态
pub struct State {
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

        let surface = unsafe { instance.create_surface(window)? };

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
