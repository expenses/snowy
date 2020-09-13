mod renderer;
use legion::*;
use renderer::*;
use grid_2d::{Coord, Grid};

#[derive(Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize, PartialOrd, Ord)]
struct TileLabel {
	label: String,
	#[serde(default)]
	rotation: Rotation,
	#[serde(default)]
	subsection: (u32, u32),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize, Debug, PartialOrd, Ord)]
enum Rotation {
	Normal,
	Minus90,
	Plus90,
	Opposite,
}

impl Rotation {
	fn to_deg(self) -> f32 {
		match self {
			Rotation::Normal => 0_f32,
			Rotation::Plus90 => 90.0,
			Rotation::Opposite => 180.0,
			Rotation::Minus90 => 270.0
		}
	}
}

impl Default for Rotation {
	fn default() -> Self {
		Self::Normal
	}
}

pub enum Image {
	Ground,
	Rocks,
	CaveEnterance,
	Cave,
	CaveWall,
	Goop,
	SnowyGround,
	WaterCorner,
	WaterEdge,
	Water,
	WaterInnerCorner,
	Egg,
	Person,
}

impl Image {
	fn coords(&self) -> (u32, u32) {
		match self {
			Self::Ground => (0, 0),
			Self::Rocks => (1, 0),
			Self::CaveEnterance => (2, 0),
			Self::Cave => (3, 0),
			Self::Goop => (0, 1),
			Self::Egg => (1, 1),
			Self::Person => (2, 1),
			Self::CaveWall => (3, 1),
			Self::SnowyGround => (0, 2),
			Self::WaterCorner => (1, 2),
			Self::WaterEdge => (2, 2),
			Self::Water => (3, 2),
			Self::WaterInnerCorner => (0, 3),
		}
	}
}

enum TileTag {
	Ground,
	Rocks,
	CaveEnterance,
	Cave,
	CaveWall,
	Goop,
	SnowyGround,
	WaterCorner,
	WaterEdge,
	Water,
	WaterInnerCorner,
}

impl TileTag {
	fn blocks_sight(&self) -> bool {
		matches!(self, Self::CaveWall)
	}

	fn blocks_movement(&self) -> bool {
		matches!(
			self,
			Self::CaveWall | Self::WaterCorner | Self::WaterEdge | Self::Water |
			Self::WaterInnerCorner | Self::Rocks
		)
	}

	fn image(&self) -> Image {
		match self {
			Self::Ground => Image::Ground,
			Self::Rocks => Image::Rocks,
			Self::CaveEnterance => Image::CaveEnterance,
			Self::Cave => Image::Cave,
			Self::Goop => Image::Goop,
			Self::CaveWall => Image::CaveWall,
			Self::SnowyGround => Image::SnowyGround,
			Self::WaterCorner => Image::WaterCorner,
			Self::WaterEdge => Image::WaterEdge,
			Self::Water => Image::Water,
			Self::WaterInnerCorner => Image::WaterInnerCorner,
		}
	}
}

struct BlocksMovement;

struct Tile {
	tag: TileTag,
	rotation: Rotation,
}

fn load_world(world: &mut World, map: Grid<TileLabel>) -> Grid<Tile> {
	Grid::new_grid_map_with_coord(map, |coord, label| {
		Tile {
			tag: match &label.label[..] {
				"ground" => TileTag::Ground,
				"rocks" => TileTag::Rocks,
				"cave_enterance" => TileTag::CaveEnterance,
				"cave" => TileTag::Cave,
				"cave_wall" => TileTag::CaveWall,
				"goop" => TileTag::Goop,
				"egg" => {
					world.push((coord, Image::Egg, BlocksMovement, Egg(255)));
					TileTag::Goop
				},
				"snowy_ground" => TileTag::SnowyGround,
				"water_corner" => TileTag::WaterCorner,
				"water_edge" => TileTag::WaterEdge,
				"water" => TileTag::Water,
				"water_inner_corner" => TileTag::WaterInnerCorner,
				other => panic!("{}", other)
			},
			rotation: label.rotation,
		}
	})
}

pub struct Camera {
	position: cgmath::Vector2<f32>,
	zoom: f32,
}

struct Player {
	position: Coord
}

struct Egg(u8);

fn main() {
	let mut world = legion::World::default();

	let map_bytes = include_bytes!("wow.ron");
	let map: Grid<TileLabel> = ron::de::from_bytes(&map_bytes[..]).unwrap();
	let map = load_world(&mut world, map);

	use winit::event_loop::*;
	use winit::event::*;

	let event_loop = EventLoop::new();
	let (mut renderer, buffer_renderer) = futures::executor::block_on(Renderer::new(&event_loop));

	let mut resources = Resources::default();

	let vis_map = Grid::new_grid_map_ref(&map, |_| Visibility::Invisible); 

	resources.insert(buffer_renderer);
	resources.insert(Camera {
		position: cgmath::vec2(0.0, 0.0),
		zoom: 64.0,
	});
	resources.insert(map);
	resources.insert(vis_map);
	resources.insert(Player {
		position: Coord { x: 2, y: 2 },
	});

	#[derive(Default)]
	struct KeyStates {
		zoom_out: bool,
		zoom_in: bool,
		up: bool,
		down: bool,
		left: bool,
		right: bool,
	} 

	let mut keys = KeyStates::default();


	let mut rendering_schedule = Schedule::builder()
		.add_system(render_map_system())
		.add_system(render_items_system())
		.add_system(render_player_system())
		.build();

	let mut turn_schedule = Schedule::builder()
		.add_system(reset_vis_map_system())
		.add_system(step_eggs_system())
		.add_system(update_vis_system())
		.build();

	turn_schedule.execute(&mut world, &mut resources);

	event_loop.run(move |event, _, control_flow| match event {
		Event::WindowEvent { event, .. } => match event {
			WindowEvent::CloseRequested => {
				*control_flow = ControlFlow::Exit;
			}
			WindowEvent::Resized(size) => {
				renderer.resize(size.width, size.height);
				*control_flow = ControlFlow::Poll;
			}
			WindowEvent::KeyboardInput {
				input:
					KeyboardInput {
						virtual_keycode: Some(code),
						state,
						..
					},
				..
			} => {
				let pressed = state == ElementState::Pressed;

				let mut move_dir = None;

				match code {
					VirtualKeyCode::X => keys.zoom_in = pressed,
					VirtualKeyCode::Z => keys.zoom_out = pressed,
					VirtualKeyCode::W => keys.up = pressed,
					VirtualKeyCode::S => keys.down = pressed,
					VirtualKeyCode::A => keys.left = pressed,
					VirtualKeyCode::D => keys.right = pressed,
					//VirtualKeyCode::Numpad1 if pressed => move_dir = Some(MovementDirection::DownLeft),
					VirtualKeyCode::Numpad2 if pressed => move_dir = Some(MovementDirection::Down),
					//VirtualKeyCode::Numpad3 if pressed => move_dir = Some(MovementDirection::DownRight),
					VirtualKeyCode::Numpad4 if pressed => move_dir = Some(MovementDirection::Left),
					VirtualKeyCode::Numpad5 if pressed => move_dir = Some(MovementDirection::StandStill),
					VirtualKeyCode::Numpad6 if pressed => move_dir = Some(MovementDirection::Right),
					//VirtualKeyCode::Numpad7 if pressed => move_dir = Some(MovementDirection::UpLeft),
					VirtualKeyCode::Numpad8 if pressed => move_dir = Some(MovementDirection::Up),
					//VirtualKeyCode::Numpad9 if pressed => move_dir = Some(MovementDirection::UpRight),
					_ => {}
				}

				if let Some(dir) = move_dir {
					let moved = {
						try_to_move_player(&mut world, &mut resources, dir)
					};
					if moved {
						turn_schedule.execute(&mut world, &mut resources);
					}
				}
			}
			_ => {}
		},
		Event::MainEventsCleared => {
			{
				let mut camera = resources.get_mut::<Camera>().unwrap();

				if keys.zoom_out {
					camera.zoom /= 1.01;
				}
				if keys.zoom_in {
					camera.zoom *= 1.01;
				}
				let move_speed = 10.0 / camera.zoom;

				if keys.up {
					camera.position.y -= move_speed;
				}
				if keys.down {
					camera.position.y += move_speed;
				}
				if keys.left {
					camera.position.x -= move_speed;
				}
				if keys.right {
					camera.position.x += move_speed;
				}
			}

			rendering_schedule.execute(&mut world, &mut resources);
			renderer.request_redraw();
		},
		Event::RedrawRequested(_) => renderer.render(&mut resources.get_mut().unwrap()),
		Event::LoopDestroyed => {}//world.fetch::<ControlsState>().save(),
		_ => {}
	});
}

#[legion::system]
fn render_map(
	#[resource] map: &Grid<Tile>,
	#[resource] camera: &Camera,
	#[resource] buffers: &mut BufferRenderer,
	#[resource] vis_map: &Grid<Visibility>,
) {
	map.enumerate()
		.zip(vis_map.iter())
		.filter(|(_, vis)| vis != &&Visibility::Invisible)
		.for_each(|((Coord { x, y }, tile), vis)| {
			let overlay = vis.overlay();

			buffers.render(cgmath::vec2(x as f32, y as f32), tile.rotation.to_deg(), &tile.tag.image(), camera, overlay);
		});
}


#[legion::system(for_each)]
fn render_items(
	position: &Coord, image: &Image,
	#[resource] buffers: &mut BufferRenderer,
	#[resource] camera: &Camera,
	#[resource] vis_map: &Grid<Visibility>,

) {
	let vis = vis_map.get_checked(*position);

	if vis == &Visibility::Visible {
		buffers.render(cgmath::vec2(position.x as f32, position.y as f32), 0.0, image, camera, vis.overlay());
	}
}

#[legion::system]
fn render_player(
	#[resource] buffers: &mut BufferRenderer,
	#[resource] player: &Player,
	#[resource] camera: &Camera,
) {
	buffers.render(cgmath::vec2(player.position.x as f32, player.position.y as f32), 0.0, &Image::Person, camera, [0.0; 4]);
}

#[legion::system]
fn reset_vis_map(
	#[resource] vis_map: &mut Grid<Visibility>,
) {
	vis_map.iter_mut().for_each(|vis| if let Visibility::Visible = vis {
		*vis = Visibility::PreviouslyVisible
	});
}

#[legion::system(for_each)]
fn step_eggs(
	entity: &Entity,
	egg: &mut Egg,
	buffer: &mut legion::systems::CommandBuffer,
) {
	egg.0 -= 1;
	if egg.0 == 0 {
		buffer.remove(*entity);
	}
}

#[legion::system]
fn update_vis(
	#[resource] player: &Player,
	#[resource] map: &Grid<Tile>,
	#[resource] vis_map: &mut Grid<Visibility>,
) {
	let position = player.position;

	let radius = 10;

	for x in position.x - radius ..= position.x + radius {
		for y in position.y - radius ..= position.y + radius {
			if let Some(vis) = vis_map.get_mut(Coord { x, y }) {
				if position.distance2(Coord { x, y }) <= (radius as u32).pow(2) {
					let is_visible = line_drawing::Bresenham::new((x, y), (position.x, position.y))
						.skip(1)
						.all(|(x, y)| !map.get_checked(Coord { x, y }).tag.blocks_sight());

					if is_visible {
						*vis = Visibility::Visible;
					}
				}
			}
		}
	}
}

#[derive(PartialEq)]
enum Visibility {
	Invisible,
	Visible,
	PreviouslyVisible
}

impl Visibility {
	fn overlay(&self) -> [f32; 4] {
		match self {
			Visibility::Invisible => [0.0, 0.0, 0.0, 1.0],
			Visibility::PreviouslyVisible => [0.0, 0.0, 0.0, 0.75],
			Visibility::Visible => [0.0; 4],
		}
	}
}

#[derive(PartialEq)]
enum MovementDirection {
	Up,
	Down,
	Left,
	Right,
	UpLeft,
	UpRight,
	DownLeft,
	DownRight,
	StandStill,
}

impl MovementDirection {
	fn relative_coord(&self) -> Coord {
		match self {
			Self::Up => Coord::new(0, -1),
			Self::Down => Coord::new(0, 1),
			Self::Left => Coord::new(-1, 0),
			Self::Right => Coord::new(1, 0),
			Self::UpLeft => Coord::new(-1, -1),
			Self::UpRight => Coord::new(1, -1),
			Self::DownLeft => Coord::new(-1, 1),
			Self::DownRight => Coord::new(1, 1),
			Self::StandStill => Coord::new(0, 0)
		}
	}
}

fn try_to_move_player(world: &mut World, resources: &mut Resources, direction: MovementDirection) -> bool {
	if direction == MovementDirection::StandStill {
		return true;
	}

	let mut player = resources.get_mut::<Player>().unwrap();
	let grid = resources.get::<Grid<Tile>>().unwrap();

	let new_coord = player.position + direction.relative_coord();


	match grid.get(new_coord) {
		None => false,
		Some(tile) => {
			let can_move = !tile.tag.blocks_movement();
			let entity_at = <(&Coord, &BlocksMovement)>::query().iter(world).any(|(coord, _)| *coord == new_coord);

			if can_move && !entity_at {
				player.position = new_coord;
			}
			can_move
		}
	}
}
