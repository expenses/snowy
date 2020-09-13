use winit::{
	event_loop::EventLoop,
	window::Window,
};

use zerocopy::*;
use wgpu::util::DeviceExt;

pub struct Renderer {
	pub swap_chain: wgpu::SwapChain,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub window: Window,
	pub pipeline: wgpu::RenderPipeline,
	pub swap_chain_desc: wgpu::SwapChainDescriptor,
	pub surface: wgpu::Surface,
	pub bind_group: wgpu::BindGroup,
	pub square_buffer: wgpu::Buffer,
	pub bind_group_layout: wgpu::BindGroupLayout,
	pub texture: wgpu::TextureView,
	pub sampler: wgpu::Sampler,
}

impl Renderer {
	pub async fn new(event_loop: &EventLoop<()>) -> (Self, BufferRenderer) {
		let window = Window::new(event_loop).unwrap();

		let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
		let surface = unsafe {
			instance.create_surface(&window)
		};

		let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
			power_preference: wgpu::PowerPreference::Default,
			compatible_surface: Some(&surface),
		}).await.unwrap();
	
		let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
			features: wgpu::Features::empty(),
			limits: wgpu::Limits::default(),
			shader_validation: true,
		}, Some(&std::path::Path::new("trace"))).await.unwrap();

		let vs_module =
			device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
	
		let fs_module =
			device.create_shader_module(wgpu::include_spirv!("shader.frag.spv"));
	
		let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
		let texture = load_png(include_bytes!("alien.png"), &device, &mut init_encoder);
		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::Repeat,
			address_mode_v: wgpu::AddressMode::Repeat,
			address_mode_w: wgpu::AddressMode::Repeat,
			mag_filter: wgpu::FilterMode::Nearest,
			min_filter: wgpu::FilterMode::Nearest,
			mipmap_filter: wgpu::FilterMode::Nearest,
			lod_min_clamp: 0.0,
			lod_max_clamp: 0.0,
			compare: None,
			anisotropy_clamp: None,
			label: None
		});

		let bind_group_layout =
			device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				entries: &[
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::SampledTexture {
							multisampled: false,
							dimension: wgpu::TextureViewDimension::D2,
							component_type: wgpu::TextureComponentType::Float,
						},
						count: None,
					},
					wgpu::BindGroupLayoutEntry {
						binding: 1,
						visibility: wgpu::ShaderStage::FRAGMENT,
						ty: wgpu::BindingType::Sampler { comparison: false },
						count: None,
					},
					wgpu::BindGroupLayoutEntry {
						binding: 2,
						visibility: wgpu::ShaderStage::VERTEX,
						ty: wgpu::BindingType::UniformBuffer {
							dynamic: false,
							min_binding_size: None,
						},
						count: None,
					}
				],
				label: None,
			});

		let window_size = window.inner_size();

		let bind_group = create_bind_group(&device, &bind_group_layout, &texture, &sampler, Uniforms::new(window_size.width, window_size.height));

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			bind_group_layouts: &[&bind_group_layout],
			push_constant_ranges: Default::default(),
			label: None,
		});

		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: None,
			layout: Some(&pipeline_layout),
			vertex_stage: wgpu::ProgrammableStageDescriptor {
				module: &vs_module,
				entry_point: "main",
			},
			fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
				module: &fs_module,
				entry_point: "main",
			}),
			rasterization_state: Some(wgpu::RasterizationStateDescriptor {
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: wgpu::CullMode::None,
				depth_bias: 0,
				depth_bias_slope_scale: 0.0,
				depth_bias_clamp: 0.0,
				clamp_depth: false,
			}),
			primitive_topology: wgpu::PrimitiveTopology::TriangleList,
			color_states: &[wgpu::ColorStateDescriptor {
				format: wgpu::TextureFormat::Bgra8Unorm,
				color_blend: wgpu::BlendDescriptor {
					src_factor: wgpu::BlendFactor::SrcAlpha,
					dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
					operation: wgpu::BlendOperation::Add,
				},
				alpha_blend: wgpu::BlendDescriptor {
					src_factor: wgpu::BlendFactor::SrcAlpha,
					dst_factor: wgpu::BlendFactor::DstAlpha,
					operation: wgpu::BlendOperation::Max,
				},
				write_mask: wgpu::ColorWrite::ALL,
			}],
			depth_stencil_state: None,
			vertex_state: wgpu::VertexStateDescriptor {
				index_format: wgpu::IndexFormat::Uint16,
				vertex_buffers: &[
					wgpu::VertexBufferDescriptor {
						stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
						step_mode: wgpu::InputStepMode::Vertex,
						attributes: &wgpu::vertex_attr_array![0 => Float2],
					},
					wgpu::VertexBufferDescriptor {
						stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
						step_mode: wgpu::InputStepMode::Instance,
						attributes: &wgpu::vertex_attr_array![1 => Float2, 2 => Float2, 3 => Float, 4 => Float2, 5 => Float4],
					}
				],
			},
			sample_count: 1,
			sample_mask: !0,
			alpha_to_coverage_enabled: false,
		});
	
		let swap_chain_desc = wgpu::SwapChainDescriptor {
			usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
			format: wgpu::TextureFormat::Bgra8Unorm,
			width: window_size.width,
			height: window_size.height,
			present_mode: wgpu::PresentMode::Fifo,
		};
	
		let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

		queue.submit(Some(init_encoder.finish()));

		let buffer_renderer = BufferRenderer {
			instances: Vec::new(),
		};

		let square_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: SQUARE.as_bytes(),
			usage: wgpu::BufferUsage::VERTEX,
		});

		let renderer = Self {
			square_buffer, swap_chain, pipeline, window, device, queue, swap_chain_desc, surface,
			bind_group, bind_group_layout, texture, sampler,
		};

		(renderer, buffer_renderer)
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		self.swap_chain_desc.width = width;
		self.swap_chain_desc.height = height;
		self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_desc);
		self.bind_group = create_bind_group(&self.device, &self.bind_group_layout, &self.texture, &self.sampler, Uniforms::new(width, height));
	}

	pub fn render(&mut self, renderer: &mut BufferRenderer) {        
		let buffers = if !renderer.instances.is_empty() {
			Some(
				self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: None,
					contents: renderer.instances.as_bytes(),
					usage: wgpu::BufferUsage::VERTEX,
				})
			)
		} else {
			None
		};

		if let Ok(frame) = self.swap_chain.get_current_frame() {
			let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: None
			});

			{
				let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
						attachment: &frame.output.view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
							store: true,
						},
					}],
					depth_stencil_attachment: None,
				});
	
				if let Some(instances) = &buffers {
					rpass.set_pipeline(&self.pipeline);
					rpass.set_bind_group(0, &self.bind_group, &[]);
	
					rpass.set_vertex_buffer(0, self.square_buffer.slice(..));
					rpass.set_vertex_buffer(1, instances.slice(..));
					rpass.draw(0 .. SQUARE.len() as u32, 0 .. renderer.instances.len() as u32);
				}
			}
	
			self.queue.submit(Some(encoder.finish()));    
		}

		renderer.instances.clear();
	}

	pub fn request_redraw(&mut self) {
		self.window.request_redraw();
	}
}

fn create_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, texture: &wgpu::TextureView, sampler: &wgpu::Sampler, uniforms: Uniforms) -> wgpu::BindGroup {
	let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: uniforms.as_bytes(),
		usage: wgpu::BufferUsage::UNIFORM
	});
	device.create_bind_group(&wgpu::BindGroupDescriptor {
		layout,
		entries: &[
			wgpu::BindGroupEntry {
				binding: 0,
				resource: wgpu::BindingResource::TextureView(texture),
			},
			wgpu::BindGroupEntry {
				binding: 1,
				resource: wgpu::BindingResource::Sampler(sampler),
			},
			wgpu::BindGroupEntry {
				binding: 2,
				resource: wgpu::BindingResource::Buffer(buffer.slice(..))
			}
		],
		label: None,
	})
}

const SQUARE: [Vertex; 6] = [
	Vertex { point: [-1.0, -1.0] },
	Vertex { point: [ 1.0, -1.0] },
	Vertex { point: [-1.0,  1.0] },

	Vertex { point: [ 1.0, -1.0] },
	Vertex { point: [-1.0,  1.0] },
	Vertex { point: [ 1.0,  1.0] },
];

#[repr(C)]
#[derive(zerocopy::AsBytes, Clone, Debug)]
pub struct Vertex {
	point: [f32; 2],
}

#[repr(C)]
#[derive(zerocopy::AsBytes, Clone, Debug)]
pub struct Instance {
	pub center: [f32; 2],
	pub dimensions: [f32; 2],
	pub rotation: f32,
	pub uv_top_left: [f32; 2],
	pub overlay: [f32; 4],
}

#[repr(C)]
#[derive(zerocopy::AsBytes, Clone, Debug)]
pub struct Uniforms {
	window_size: [f32; 2],
}

impl Uniforms {
	fn new(width: u32, height: u32) -> Self {
		Self {
			window_size: [width as f32, height as f32],
		}
	}
}

use crate::Image;
use crate::Camera;

pub struct BufferRenderer {
	instances: Vec<Instance>,
}

impl BufferRenderer {
	pub fn render(
		&mut self, tile_position: cgmath::Vector2<f32>, rotation_deg: f32, image: &Image,
		camera: &Camera, overlay: [f32; 4],
	) {
		let mut position = (camera.position - tile_position) * camera.zoom;
		position.x = -position.x;

		self.instances.push(Instance {
			center: position.into(),
			dimensions: [0.5 * camera.zoom, 0.5 * camera.zoom],
			rotation: rotation_deg.to_radians(),
			uv_top_left: {
				let (x, y) = image.coords();
				[x as f32 /  4.0, y as f32 / 4.0]
			},
			overlay,
		});
	}
}

fn load_png(bytes: &'static [u8], device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) -> wgpu::TextureView {
	let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap()
		.into_rgba();

	let temp_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
		label: None,
		contents: &*image,
		usage: wgpu::BufferUsage::COPY_SRC,
	});

	let texture_extent = wgpu::Extent3d {
		width: image.width(),
		height: image.height(),
		depth: 1,
	};

	let texture = device.create_texture(&wgpu::TextureDescriptor {
		size: texture_extent,
		mip_level_count: 1,
		sample_count: 1,
		dimension: wgpu::TextureDimension::D2,
		format: wgpu::TextureFormat::Rgba8Unorm,
		usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
		label: None,
	});

	encoder.copy_buffer_to_texture(
		wgpu::BufferCopyView {
			buffer: &temp_buf,
			layout: wgpu::TextureDataLayout {
				offset: 0,
				bytes_per_row: 4 * image.width(),
				rows_per_image: 0,
			},
		},
		wgpu::TextureCopyView {
			texture: &texture,
			mip_level: 0,
			//array_layer: 0,
			origin: wgpu::Origin3d::ZERO,
		},
		texture_extent,
	);

	texture.create_view(&wgpu::TextureViewDescriptor::default())
}
