use nightshade::ecs::prefab::{Prefab, PrefabNode};
use nightshade::ecs::world::components::Line;
use nightshade::ecs::world::{
    spawn_instanced_mesh_with_material, GLOBAL_TRANSFORM, LINES, LOCAL_TRANSFORM,
    LOCAL_TRANSFORM_DIRTY, VISIBILITY,
};
use nightshade::prelude::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};

use nightshade::ecs::picking::queries::PickingRay;

const HEXAGON_TILES_GLB: &[u8] = include_bytes!("../assets/hexagon_tiles.glb");
const GRASS_GLB: &[u8] = include_bytes!("../assets/grass.glb");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    launch(HexWarState::default())?;
    Ok(())
}

#[derive(Clone)]
struct MapGenParams {
    grid_radius: i32,
    noise_scale: f32,
    coast_threshold: f32,
    coast_falloff: f32,
    lake_threshold: f32,
    desert_temp_threshold: f32,
    desert_moisture_threshold: f32,
    forest_moisture_threshold: f32,
    forest_elevation_threshold: f32,
    num_tributaries: u32,
    river_width: i32,
    meander_chance: u32,
}

impl Default for MapGenParams {
    fn default() -> Self {
        Self {
            grid_radius: 30,
            noise_scale: 0.06,
            coast_threshold: 0.38,
            coast_falloff: 0.12,
            lake_threshold: 0.65,
            desert_temp_threshold: 0.55,
            desert_moisture_threshold: 0.45,
            forest_moisture_threshold: 0.5,
            forest_elevation_threshold: 0.45,
            num_tributaries: 3,
            river_width: 2,
            meander_chance: 30,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct HexCoord {
    column: i32,
    row: i32,
}

impl HexCoord {
    fn new(column: i32, row: i32) -> Self {
        Self { column, row }
    }

    fn to_cube(self) -> (i32, i32, i32) {
        let x = self.column;
        let z = self.row - (self.column - (self.column & 1)) / 2;
        let y = -x - z;
        (x, y, z)
    }

    fn tiles_in_range(&self, range: i32) -> Vec<HexCoord> {
        let mut result = Vec::new();
        for distance in 1..=range {
            let tiles_at_distance = self.tiles_at_distance(distance);
            result.extend(tiles_at_distance);
        }
        result
    }

    fn tiles_at_distance(&self, distance: i32) -> Vec<HexCoord> {
        if distance == 0 {
            return vec![*self];
        }
        let mut result = Vec::new();
        let (cx, cy, cz) = self.to_cube();
        for x in -distance..=distance {
            for y in (-distance).max(-x - distance)..=(distance).min(-x + distance) {
                let z = -x - y;
                if x.abs() + y.abs() + z.abs() == distance * 2 {
                    let coord = HexCoord::from_cube(cx + x, cy + y, cz + z);
                    result.push(coord);
                }
            }
        }
        result
    }

    fn from_cube(x: i32, _y: i32, z: i32) -> Self {
        let column = x;
        let row = z + (x - (x & 1)) / 2;
        Self { column, row }
    }
}

struct Unit {
    entity: Entity,
    hex_coord: HexCoord,
    movement_range: i32,
}

struct UnitMovement {
    entity: Entity,
    start_position: Vec3,
    end_position: Vec3,
    progress: f32,
    speed: f32,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum SelectionState {
    #[default]
    None,
    UnitSelected(Entity),
}

struct InstancedTileGroup {
    entity: Entity,
}

struct HexWarState {
    coord_to_tile_type: HashMap<HexCoord, TileType>,
    instanced_tile_groups: Vec<InstancedTileGroup>,
    lines_entity: Option<Entity>,
    hovered_tile: Option<HexCoord>,
    tile_prefabs: Vec<Prefab>,
    hex_width: f32,
    hex_depth: f32,
    rng_seed: u32,
    params: MapGenParams,
    needs_regeneration: bool,
    units: Vec<Unit>,
    selected_unit: Option<Entity>,
    selection_state: SelectionState,
    range_lines_entity: Option<Entity>,
    hovered_unit: Option<Entity>,
    valid_move_tiles: HashSet<HexCoord>,
    fps_text_entity: Option<Entity>,
    active_movements: Vec<UnitMovement>,
}

impl Default for HexWarState {
    fn default() -> Self {
        Self {
            coord_to_tile_type: HashMap::new(),
            instanced_tile_groups: Vec::new(),
            lines_entity: None,
            hovered_tile: None,
            tile_prefabs: Vec::new(),
            hex_width: 0.0,
            hex_depth: 0.0,
            rng_seed: 0,
            params: MapGenParams::default(),
            needs_regeneration: false,
            units: Vec::new(),
            selected_unit: None,
            selection_state: SelectionState::None,
            range_lines_entity: None,
            hovered_unit: None,
            valid_move_tiles: HashSet::new(),
            fps_text_entity: None,
            active_movements: Vec::new(),
        }
    }
}

fn spawn_unit(world: &mut World, position: Vec3) -> Entity {
    let unit_radius = 30.0;
    let unit_entity = spawn_mesh(
        world,
        "Sphere",
        nalgebra_glm::vec3(position.x, position.y + unit_radius + 10.0, position.z),
        nalgebra_glm::vec3(unit_radius, unit_radius, unit_radius),
    );

    if let Some(material) = world.get_material_mut(unit_entity) {
        material.base_color = [0.2, 0.6, 1.0, 1.0];
    }

    world.add_components(unit_entity, BOUNDING_VOLUME);
    let bounding_volume = BoundingVolume {
        obb: OrientedBoundingBox {
            center: nalgebra_glm::vec3(0.0, 0.0, 0.0),
            half_extents: nalgebra_glm::vec3(unit_radius, unit_radius, unit_radius),
            orientation: nalgebra_glm::quat_identity(),
        },
        sphere_radius: unit_radius,
    };
    world.set_bounding_volume(unit_entity, bounding_volume);

    unit_entity
}

fn generate_range_circle_lines(
    tiles_in_range: &[HexCoord],
    hex_width: f32,
    hex_depth: f32,
    color: Vec4,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let y_offset = 10.0;

    for coord in tiles_in_range {
        let tile_center = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
        let hex_lines = generate_hex_outline(
            tile_center,
            hex_width,
            hex_depth,
            y_offset,
        );
        for mut line in hex_lines {
            line.color = color;
            lines.push(line);
        }
    }

    lines
}

fn generate_hex_outline(center: Vec3, hex_width: f32, hex_height: f32, y_offset: f32) -> Vec<Line> {
    let mut lines = Vec::new();
    let is_flat_top = hex_width > hex_height;
    let color = nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0);

    let vertices: Vec<Vec3> = if is_flat_top {
        let half_width = hex_width / 2.0;
        let quarter_width = hex_width / 4.0;
        let half_height = hex_height / 2.0;
        vec![
            nalgebra_glm::vec3(center.x + half_width, y_offset, center.z),
            nalgebra_glm::vec3(center.x + quarter_width, y_offset, center.z + half_height),
            nalgebra_glm::vec3(center.x - quarter_width, y_offset, center.z + half_height),
            nalgebra_glm::vec3(center.x - half_width, y_offset, center.z),
            nalgebra_glm::vec3(center.x - quarter_width, y_offset, center.z - half_height),
            nalgebra_glm::vec3(center.x + quarter_width, y_offset, center.z - half_height),
        ]
    } else {
        let half_width = hex_width / 2.0;
        let half_height = hex_height / 2.0;
        let quarter_height = hex_height / 4.0;
        vec![
            nalgebra_glm::vec3(center.x, y_offset, center.z - half_height),
            nalgebra_glm::vec3(center.x + half_width, y_offset, center.z - quarter_height),
            nalgebra_glm::vec3(center.x + half_width, y_offset, center.z + quarter_height),
            nalgebra_glm::vec3(center.x, y_offset, center.z + half_height),
            nalgebra_glm::vec3(center.x - half_width, y_offset, center.z + quarter_height),
            nalgebra_glm::vec3(center.x - half_width, y_offset, center.z - quarter_height),
        ]
    };

    for vertex_index in 0..6 {
        let start = vertices[vertex_index];
        let end = vertices[(vertex_index + 1) % 6];
        lines.push(Line { start, end, color });
    }

    lines
}

fn hex_to_world_position(column: i32, row: i32, hex_width: f32, hex_height: f32) -> Vec3 {
    let is_flat_top = hex_width > hex_height;

    if is_flat_top {
        let horizontal_spacing = hex_width * 0.75;
        let vertical_spacing = hex_height;

        let offset = if column.abs() % 2 != 0 {
            vertical_spacing * 0.5
        } else {
            0.0
        };

        nalgebra_glm::vec3(
            column as f32 * horizontal_spacing,
            0.0,
            row as f32 * vertical_spacing + offset,
        )
    } else {
        let horizontal_spacing = hex_width;
        let vertical_spacing = hex_height * 0.75;

        let offset = if row.abs() % 2 != 0 {
            horizontal_spacing * 0.5
        } else {
            0.0
        };

        nalgebra_glm::vec3(
            column as f32 * horizontal_spacing + offset,
            0.0,
            row as f32 * vertical_spacing,
        )
    }
}

fn world_to_hex(world_x: f32, world_z: f32, hex_width: f32, hex_height: f32) -> HexCoord {
    let is_flat_top = hex_width > hex_height;

    if is_flat_top {
        let horizontal_spacing = hex_width * 0.75;
        let vertical_spacing = hex_height;

        let approx_column = (world_x / horizontal_spacing).round() as i32;
        let offset = if approx_column.abs() % 2 != 0 {
            vertical_spacing * 0.5
        } else {
            0.0
        };
        let approx_row = ((world_z - offset) / vertical_spacing).round() as i32;

        let mut best_coord = HexCoord::new(approx_column, approx_row);
        let mut best_dist = f32::MAX;

        for dc in -1..=1 {
            for dr in -1..=1 {
                let candidate = HexCoord::new(approx_column + dc, approx_row + dr);
                let candidate_pos =
                    hex_to_world_position(candidate.column, candidate.row, hex_width, hex_height);
                let dist_sq = (candidate_pos.x - world_x).powi(2)
                    + (candidate_pos.z - world_z).powi(2);
                if dist_sq < best_dist {
                    best_dist = dist_sq;
                    best_coord = candidate;
                }
            }
        }
        best_coord
    } else {
        let horizontal_spacing = hex_width;
        let vertical_spacing = hex_height * 0.75;

        let approx_row = (world_z / vertical_spacing).round() as i32;
        let offset = if approx_row.abs() % 2 != 0 {
            horizontal_spacing * 0.5
        } else {
            0.0
        };
        let approx_column = ((world_x - offset) / horizontal_spacing).round() as i32;

        let mut best_coord = HexCoord::new(approx_column, approx_row);
        let mut best_dist = f32::MAX;

        for dc in -1..=1 {
            for dr in -1..=1 {
                let candidate = HexCoord::new(approx_column + dc, approx_row + dr);
                let candidate_pos =
                    hex_to_world_position(candidate.column, candidate.row, hex_width, hex_height);
                let dist_sq = (candidate_pos.x - world_x).powi(2)
                    + (candidate_pos.z - world_z).powi(2);
                if dist_sq < best_dist {
                    best_dist = dist_sq;
                    best_coord = candidate;
                }
            }
        }
        best_coord
    }
}

fn find_node_by_name<'a>(nodes: &'a [PrefabNode], name: &str) -> Option<&'a PrefabNode> {
    for node in nodes {
        if let Some(node_name) = &node.components.name
            && node_name.0 == name
        {
            return Some(node);
        }
        if let Some(found) = find_node_by_name(&node.children, name) {
            return Some(found);
        }
    }
    None
}

struct ExtractedMesh {
    mesh_name: String,
    material: Material,
    local_transform: LocalTransform,
}

fn extract_meshes_from_prefab(prefab: &Prefab) -> Vec<ExtractedMesh> {
    let mut meshes = Vec::new();
    for node in &prefab.root_nodes {
        extract_meshes_from_node(node, LocalTransform::default(), &mut meshes);
    }
    meshes
}

fn extract_meshes_from_node(
    node: &PrefabNode,
    parent_transform: LocalTransform,
    meshes: &mut Vec<ExtractedMesh>,
) {
    let combined_translation = parent_transform.translation
        + nalgebra_glm::quat_rotate_vec3(&parent_transform.rotation, &nalgebra_glm::vec3(
            node.local_transform.translation.x * parent_transform.scale.x,
            node.local_transform.translation.y * parent_transform.scale.y,
            node.local_transform.translation.z * parent_transform.scale.z,
        ));
    let combined_rotation = parent_transform.rotation * node.local_transform.rotation;
    let combined_scale = nalgebra_glm::vec3(
        parent_transform.scale.x * node.local_transform.scale.x,
        parent_transform.scale.y * node.local_transform.scale.y,
        parent_transform.scale.z * node.local_transform.scale.z,
    );

    let combined_transform = LocalTransform {
        translation: combined_translation,
        rotation: combined_rotation,
        scale: combined_scale,
    };

    if let Some(render_mesh) = &node.components.render_mesh {
        let material = node.components.material.clone().unwrap_or_default();
        meshes.push(ExtractedMesh {
            mesh_name: render_mesh.name.clone(),
            material,
            local_transform: combined_transform,
        });
    }

    for child in &node.children {
        extract_meshes_from_node(child, combined_transform, meshes);
    }
}

fn simple_noise(x: f32, y: f32, seed: u32) -> f32 {
    let x_int = x.floor() as i32;
    let y_int = y.floor() as i32;
    let x_frac = x - x.floor();
    let y_frac = y - y.floor();

    fn hash(x: i32, y: i32, seed: u32) -> f32 {
        let n = (x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed as i32) as u32;
        let n = n.wrapping_mul(n).wrapping_mul(n);
        (n as f32) / (u32::MAX as f32)
    }

    let v00 = hash(x_int, y_int, seed);
    let v10 = hash(x_int + 1, y_int, seed);
    let v01 = hash(x_int, y_int + 1, seed);
    let v11 = hash(x_int + 1, y_int + 1, seed);

    let sx = x_frac * x_frac * (3.0 - 2.0 * x_frac);
    let sy = y_frac * y_frac * (3.0 - 2.0 * y_frac);

    let a = v00 + sx * (v10 - v00);
    let b = v01 + sx * (v11 - v01);
    a + sy * (b - a)
}

fn fbm_noise(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for octave in 0..octaves {
        value += amplitude
            * simple_noise(
                x * frequency,
                y * frequency,
                seed.wrapping_add(octave * 1000),
            );
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TileType {
    Sea,
    Forest,
    Desert,
    ForestLake,
    Farm,
    CropFarm,
    ClayPit,
    StartingTile,
    Grass,
}

fn generate_river_set(params: &MapGenParams, seed: u32) -> HashSet<(i32, i32)> {
    let mut river_tiles = HashSet::new();
    let mut rng_state = seed;
    let grid_radius = params.grid_radius;

    rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
    let start_side = rng_state % 4;
    let (mut current_col, mut current_row) = match start_side {
        0 => (
            -grid_radius,
            ((rng_state >> 8) as i32 % (grid_radius * 2)) - grid_radius,
        ),
        1 => (
            grid_radius,
            ((rng_state >> 8) as i32 % (grid_radius * 2)) - grid_radius,
        ),
        2 => (
            ((rng_state >> 8) as i32 % (grid_radius * 2)) - grid_radius,
            -grid_radius,
        ),
        _ => (
            ((rng_state >> 8) as i32 % (grid_radius * 2)) - grid_radius,
            grid_radius,
        ),
    };

    let (target_col, target_row) = match (start_side + 2) % 4 {
        0 => (
            -grid_radius,
            ((rng_state >> 16) as i32 % (grid_radius * 2)) - grid_radius,
        ),
        1 => (
            grid_radius,
            ((rng_state >> 16) as i32 % (grid_radius * 2)) - grid_radius,
        ),
        2 => (
            ((rng_state >> 16) as i32 % (grid_radius * 2)) - grid_radius,
            -grid_radius,
        ),
        _ => (
            ((rng_state >> 16) as i32 % (grid_radius * 2)) - grid_radius,
            grid_radius,
        ),
    };

    for _ in 0..300 {
        for dx in -params.river_width..=params.river_width {
            for dy in -params.river_width..=params.river_width {
                river_tiles.insert((current_col + dx, current_row + dy));
            }
        }

        let dx = target_col - current_col;
        let dy = target_row - current_row;

        if dx.abs() <= 2 && dy.abs() <= 2 {
            break;
        }

        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let wobble = ((rng_state >> 16) % 7) as i32 - 3;
        let meander = ((rng_state >> 8) % 100) < params.meander_chance;

        if meander {
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(54321);
            let dir = (rng_state % 4) as i32;
            match dir {
                0 => current_col += 1,
                1 => current_col -= 1,
                2 => current_row += 1,
                _ => current_row -= 1,
            }
        } else if dx.abs() > dy.abs() {
            current_col += dx.signum();
            current_row += wobble.clamp(-1, 1);
        } else {
            current_row += dy.signum();
            current_col += wobble.clamp(-1, 1);
        }

        current_col = current_col.clamp(-grid_radius, grid_radius);
        current_row = current_row.clamp(-grid_radius, grid_radius);
    }

    for tributary_index in 0..params.num_tributaries {
        rng_state = rng_state
            .wrapping_mul(1103515245)
            .wrapping_add(tributary_index * 1000);

        let start_angle = (rng_state as f32 / u32::MAX as f32) * std::f32::consts::TAU;
        let start_radius = grid_radius as f32 * 0.7;
        let mut trib_col = (start_angle.cos() * start_radius) as i32;
        let mut trib_row = (start_angle.sin() * start_radius) as i32;

        for _ in 0..80 {
            river_tiles.insert((trib_col, trib_row));

            let mut closest_dist = i32::MAX;
            let mut closest = (0, 0);
            for &(rc, rr) in &river_tiles {
                let dist = (rc - trib_col).abs() + (rr - trib_row).abs();
                if dist > 0 && dist < closest_dist {
                    closest_dist = dist;
                    closest = (rc, rr);
                }
            }

            if closest_dist <= 2 {
                break;
            }

            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let wobble = ((rng_state >> 16) % 3) as i32 - 1;

            let dx = closest.0 - trib_col;
            let dy = closest.1 - trib_row;

            if dx.abs() > dy.abs() {
                trib_col += dx.signum();
                trib_row += wobble;
            } else {
                trib_row += dy.signum();
                trib_col += wobble;
            }

            trib_col = trib_col.clamp(-grid_radius, grid_radius);
            trib_row = trib_row.clamp(-grid_radius, grid_radius);
        }
    }

    river_tiles
}

fn generate_tile_type(
    column: i32,
    row: i32,
    params: &MapGenParams,
    seed: u32,
    river_tiles: &HashSet<(i32, i32)>,
) -> TileType {
    let x = column as f32 * params.noise_scale;
    let y = row as f32 * params.noise_scale;

    let elevation = fbm_noise(x, y, seed, 4);
    let moisture = fbm_noise(x + 100.0, y + 100.0, seed.wrapping_add(5000), 4);
    let temperature = fbm_noise(x + 200.0, y + 200.0, seed.wrapping_add(10000), 3);

    let distance_from_center =
        ((column * column + row * row) as f32).sqrt() / params.grid_radius as f32;

    let is_river = river_tiles.contains(&(column, row));

    let lake_noise = fbm_noise(x * 1.5, y * 1.5, seed.wrapping_add(40000), 3);
    let is_lake = lake_noise > params.lake_threshold && elevation > 0.4;

    let coast_threshold = params.coast_threshold + distance_from_center * params.coast_falloff;

    let variety_noise = simple_noise(x * 4.0, y * 4.0, seed.wrapping_add(50000));
    let resource_noise = simple_noise(x * 6.0, y * 6.0, seed.wrapping_add(60000));

    if elevation < coast_threshold {
        TileType::Sea
    } else if is_river || is_lake {
        TileType::ForestLake
    } else if temperature > params.desert_temp_threshold
        && moisture < params.desert_moisture_threshold
    {
        TileType::Desert
    } else if moisture > params.forest_moisture_threshold
        && elevation > params.forest_elevation_threshold
    {
        if resource_noise > 0.85 {
            TileType::ClayPit
        } else {
            TileType::Forest
        }
    } else if variety_noise < 0.05 {
        TileType::StartingTile
    } else if variety_noise < 0.15 {
        TileType::Forest
    } else if variety_noise < 0.20 {
        TileType::Farm
    } else if variety_noise < 0.25 {
        TileType::CropFarm
    } else if resource_noise > 0.9 {
        TileType::ClayPit
    } else {
        TileType::Grass
    }
}

fn tile_type_to_prefab_index(tile_type: TileType) -> usize {
    match tile_type {
        TileType::Forest => 0,
        TileType::Sea => 1,
        TileType::Desert => 2,
        TileType::ForestLake => 3,
        TileType::Farm => 4,
        TileType::CropFarm => 5,
        TileType::ClayPit => 6,
        TileType::StartingTile => 7,
        TileType::Grass => 8,
    }
}

fn create_instanced_tiles(
    world: &mut World,
    tile_prefabs: &[Prefab],
    tile_positions: &[(HexCoord, TileType)],
    hex_width: f32,
    hex_depth: f32,
) -> Vec<InstancedTileGroup> {
    let prefab_meshes: Vec<Vec<ExtractedMesh>> = tile_prefabs
        .iter()
        .map(extract_meshes_from_prefab)
        .collect();

    let mut mesh_instances: HashMap<(String, u64), (Material, Vec<InstanceTransform>)> =
        HashMap::new();

    for (coord, tile_type) in tile_positions {
        let prefab_index = tile_type_to_prefab_index(*tile_type);
        let extracted_meshes = &prefab_meshes[prefab_index];

        let tile_world_pos =
            hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);

        for extracted in extracted_meshes {
            let mat_hash = material_hash(&extracted.material);
            let key = (extracted.mesh_name.clone(), mat_hash);

            let instance = InstanceTransform::new(
                nalgebra_glm::vec3(
                    tile_world_pos.x + extracted.local_transform.translation.x,
                    tile_world_pos.y + extracted.local_transform.translation.y,
                    tile_world_pos.z + extracted.local_transform.translation.z,
                ),
                extracted.local_transform.rotation,
                extracted.local_transform.scale,
            );

            mesh_instances
                .entry(key)
                .or_insert_with(|| (extracted.material.clone(), Vec::new()))
                .1
                .push(instance);
        }
    }

    let mut instanced_groups = Vec::new();
    for ((mesh_name, _), (material, instances)) in mesh_instances {
        if instances.is_empty() {
            continue;
        }
        let entity = spawn_instanced_mesh_with_material(world, &mesh_name, instances, material);
        instanced_groups.push(InstancedTileGroup { entity });
    }

    instanced_groups
}

fn material_hash(material: &Material) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    material.base_color[0].to_bits().hash(&mut hasher);
    material.base_color[1].to_bits().hash(&mut hasher);
    material.base_color[2].to_bits().hash(&mut hasher);
    material.base_color[3].to_bits().hash(&mut hasher);
    material.roughness.to_bits().hash(&mut hasher);
    material.metallic.to_bits().hash(&mut hasher);
    if let Some(texture) = &material.base_texture {
        texture.as_str().hash(&mut hasher);
    }
    hasher.finish()
}

impl HexWarState {
    fn select_unit(&mut self, world: &mut World, unit_entity: Entity) {
        self.selected_unit = Some(unit_entity);
        self.selection_state = SelectionState::UnitSelected(unit_entity);

        if let Some(material) = world.get_material_mut(unit_entity) {
            material.base_color = [1.0, 0.8, 0.2, 1.0];
        }

        let unit_index = self.units.iter().position(|u| u.entity == unit_entity);
        if let Some(index) = unit_index {
            let unit = &self.units[index];
            let tiles_in_range = unit.hex_coord.tiles_in_range(unit.movement_range);

            self.valid_move_tiles.clear();
            for coord in &tiles_in_range {
                if self.coord_to_tile_type.contains_key(coord) {
                    let is_occupied = self.units.iter().any(|u| u.hex_coord == *coord);
                    if !is_occupied {
                        self.valid_move_tiles.insert(*coord);
                    }
                }
            }

            let valid_coords: Vec<HexCoord> = self.valid_move_tiles.iter().copied().collect();
            let range_lines = generate_range_circle_lines(
                &valid_coords,
                self.hex_width,
                self.hex_depth,
                nalgebra_glm::vec4(1.0, 1.0, 0.0, 1.0),
            );

            if let Some(range_entity) = self.range_lines_entity {
                if let Some(lines_component) = world.get_lines_mut(range_entity) {
                    lines_component.lines = range_lines;
                    lines_component.mark_dirty();
                }
                if let Some(visibility) = world.get_visibility_mut(range_entity) {
                    visibility.visible = true;
                }
            }
        }
    }

    fn clear_selection(&mut self, world: &mut World) {
        if let Some(selected) = self.selected_unit
            && let Some(material) = world.get_material_mut(selected) {
                material.base_color = [0.2, 0.6, 1.0, 1.0];
            }

        if let Some(range_entity) = self.range_lines_entity
            && let Some(visibility) = world.get_visibility_mut(range_entity) {
                visibility.visible = false;
            }

        self.selected_unit = None;
        self.selection_state = SelectionState::None;
        self.valid_move_tiles.clear();
    }

    fn move_unit_to(&mut self, world: &mut World, unit_entity: Entity, destination: HexCoord) {
        let unit_index = self.units.iter().position(|u| u.entity == unit_entity);
        if let Some(index) = unit_index {
            self.units[index].hex_coord = destination;

            let end_position = hex_to_world_position(
                destination.column,
                destination.row,
                self.hex_width,
                self.hex_depth,
            );

            if let Some(transform) = world.get_local_transform(unit_entity) {
                let start_position = transform.translation;
                let unit_radius = 30.0;
                let end_position_with_height = nalgebra_glm::vec3(
                    end_position.x,
                    end_position.y + unit_radius + 10.0,
                    end_position.z,
                );

                self.active_movements.push(UnitMovement {
                    entity: unit_entity,
                    start_position,
                    end_position: end_position_with_height,
                    progress: 0.0,
                    speed: 2.0,
                });
            }
        }
    }

    fn update_unit_movements(&mut self, world: &mut World, delta_time: f32) {
        let mut completed_indices = Vec::new();

        for (movement_index, movement) in self.active_movements.iter_mut().enumerate() {
            movement.progress += delta_time * movement.speed;

            let t = movement.progress.clamp(0.0, 1.0);
            let smooth_t = t * t * (3.0 - 2.0 * t);

            let current_position = nalgebra_glm::lerp(
                &movement.start_position,
                &movement.end_position,
                smooth_t,
            );

            if let Some(transform) = world.get_local_transform_mut(movement.entity) {
                transform.translation = current_position;
                world.set_local_transform_dirty(movement.entity, LocalTransformDirty);
            }

            if movement.progress >= 1.0 {
                completed_indices.push(movement_index);
            }
        }

        for movement_index in completed_indices.into_iter().rev() {
            self.active_movements.swap_remove(movement_index);
        }
    }
}

impl State for HexWarState {
    fn title(&self) -> &str {
        "Hex War"
    }

    fn initialize(&mut self, world: &mut World) {
        world.resources.user_interface.enabled = true;
        world.resources.graphics.show_grid = false;
        world.resources.graphics.show_skybox = true;

        spawn_sun(world);

        let load_result = nightshade::ecs::prefab::import_gltf_from_bytes(HEXAGON_TILES_GLB);
        let grass_load_result = nightshade::ecs::prefab::import_gltf_from_bytes(GRASS_GLB);

        match (load_result, grass_load_result) {
            (Ok(result), Ok(grass_result)) => {
                for (name, (rgba_data, width, height)) in result.textures {
                    world.queue_command(WorldCommand::LoadTexture {
                        name,
                        rgba_data,
                        width,
                        height,
                    });
                }

                for (name, (rgba_data, width, height)) in grass_result.textures {
                    world.queue_command(WorldCommand::LoadTexture {
                        name,
                        rgba_data,
                        width,
                        height,
                    });
                }

                for (name, mesh) in result.meshes {
                    world.resources.mesh_cache.insert(name, mesh);
                }

                for (name, mesh) in grass_result.meshes {
                    world.resources.mesh_cache.insert(name, mesh);
                }

                let tile_names = [
                    "normal forest",
                    "sea",
                    "dessert",
                    "forestlake",
                    "tile animalFarm",
                    "tile  crop Farm",
                    "clay pit",
                    "startingTile",
                ];

                let mut tile_prefabs: Vec<Prefab> = Vec::new();
                for tile_name in &tile_names {
                    if let Some(node) = result
                        .prefabs
                        .iter()
                        .find_map(|prefab| find_node_by_name(&prefab.root_nodes, tile_name))
                    {
                        let mut zeroed_node = node.clone();
                        zeroed_node.local_transform.translation = nalgebra_glm::vec3(0.0, 0.0, 0.0);
                        tile_prefabs.push(Prefab {
                            name: tile_name.to_string(),
                            root_nodes: vec![zeroed_node],
                        });
                    }
                }

                if tile_prefabs.is_empty() {
                    log::error!("No tile prefabs found!");
                    return;
                }

                fn calculate_prefab_bounds(
                    prefab: &Prefab,
                    mesh_cache: &nightshade::ecs::prefab::MeshCache,
                ) -> Option<(f32, f32, f32, f32)> {
                    let mut min_x = f32::MAX;
                    let mut max_x = f32::MIN;
                    let mut min_z = f32::MAX;
                    let mut max_z = f32::MIN;

                    fn find_all_mesh_bounds(
                        node: &PrefabNode,
                        parent_scale: Vec3,
                        min_x: &mut f32,
                        max_x: &mut f32,
                        min_z: &mut f32,
                        max_z: &mut f32,
                        mesh_cache: &nightshade::ecs::prefab::MeshCache,
                    ) {
                        let current_scale = nalgebra_glm::vec3(
                            parent_scale.x * node.local_transform.scale.x,
                            parent_scale.y * node.local_transform.scale.y,
                            parent_scale.z * node.local_transform.scale.z,
                        );

                        if let Some(render_mesh) = &node.components.render_mesh
                            && let Some(mesh) = mesh_cache.get(&render_mesh.name)
                        {
                            for vertex in &mesh.vertices {
                                let scaled_x = vertex.position[0] * current_scale.x;
                                let scaled_z = vertex.position[2] * current_scale.z;
                                *min_x = min_x.min(scaled_x);
                                *max_x = max_x.max(scaled_x);
                                *min_z = min_z.min(scaled_z);
                                *max_z = max_z.max(scaled_z);
                            }
                        }

                        for child in &node.children {
                            find_all_mesh_bounds(
                                child,
                                current_scale,
                                min_x,
                                max_x,
                                min_z,
                                max_z,
                                mesh_cache,
                            );
                        }
                    }

                    for node in &prefab.root_nodes {
                        find_all_mesh_bounds(
                            node,
                            nalgebra_glm::vec3(1.0, 1.0, 1.0),
                            &mut min_x,
                            &mut max_x,
                            &mut min_z,
                            &mut max_z,
                            mesh_cache,
                        );
                    }

                    if min_x != f32::MAX {
                        Some((min_x, max_x, min_z, max_z))
                    } else {
                        None
                    }
                }

                let (hex_width, hex_depth) = {
                    let mut target_width = 173.205f32;
                    let mut target_depth = 200.0f32;

                    if let Some(first_prefab) = tile_prefabs.first()
                        && let Some((min_x, max_x, min_z, max_z)) =
                            calculate_prefab_bounds(first_prefab, &world.resources.mesh_cache)
                    {
                        target_width = max_x - min_x;
                        target_depth = max_z - min_z;
                        log::info!(
                            "Tile dimensions from hexagon_tiles: width={}, depth={}",
                            target_width,
                            target_depth
                        );
                    }

                    (target_width, target_depth)
                };

                if let Some(grass_prefab) = grass_result.prefabs.first() {
                    let mut grass_prefab_clone = grass_prefab.clone();
                    grass_prefab_clone.name = "grass".to_string();

                    if let Some((grass_min_x, grass_max_x, grass_min_z, grass_max_z)) =
                        calculate_prefab_bounds(&grass_prefab_clone, &world.resources.mesh_cache)
                    {
                        let grass_width = grass_max_x - grass_min_x;
                        let grass_depth = grass_max_z - grass_min_z;

                        if grass_width > 0.001 && grass_depth > 0.001 {
                            let scale_x = hex_width / grass_width;
                            let scale_z = hex_depth / grass_depth;
                            let scale = scale_x.min(scale_z);

                            log::info!(
                                "Grass bounds: {}x{}, scaling by {} to match hex tiles",
                                grass_width,
                                grass_depth,
                                scale
                            );

                            for root_node in &mut grass_prefab_clone.root_nodes {
                                root_node.local_transform.translation =
                                    nalgebra_glm::vec3(0.0, 0.0, 0.0);
                                root_node.local_transform.scale = nalgebra_glm::vec3(
                                    root_node.local_transform.scale.x * scale,
                                    root_node.local_transform.scale.y * scale,
                                    root_node.local_transform.scale.z * scale,
                                );
                            }
                        }
                    } else {
                        for root_node in &mut grass_prefab_clone.root_nodes {
                            root_node.local_transform.translation =
                                nalgebra_glm::vec3(0.0, 0.0, 0.0);
                        }
                    }

                    tile_prefabs.push(grass_prefab_clone);
                }

                self.tile_prefabs = tile_prefabs;

                self.hex_width = hex_width;
                self.hex_depth = hex_depth;
                self.rng_seed = rand::rng().random();

                let river_tiles = generate_river_set(&self.params, self.rng_seed);
                let mut all_hex_lines: Vec<Line> = Vec::new();
                let mut tile_positions: Vec<(HexCoord, TileType)> = Vec::new();

                for row in -self.params.grid_radius..=self.params.grid_radius {
                    for column in -self.params.grid_radius..=self.params.grid_radius {
                        let tile_type = generate_tile_type(
                            column,
                            row,
                            &self.params,
                            self.rng_seed,
                            &river_tiles,
                        );
                        let hex_coord = HexCoord::new(column, row);
                        tile_positions.push((hex_coord, tile_type));
                        self.coord_to_tile_type.insert(hex_coord, tile_type);

                        let position = hex_to_world_position(column, row, hex_width, hex_depth);
                        let hex_lines = generate_hex_outline(position, hex_width, hex_depth, 5.0);
                        all_hex_lines.extend(hex_lines);
                    }
                }

                self.instanced_tile_groups = create_instanced_tiles(
                    world,
                    &self.tile_prefabs,
                    &tile_positions,
                    hex_width,
                    hex_depth,
                );

                let lines_entity = world.spawn_entities(
                    LINES | VISIBILITY | LOCAL_TRANSFORM | GLOBAL_TRANSFORM | LOCAL_TRANSFORM_DIRTY,
                    1,
                )[0];
                if let Some(lines_component) = world.get_lines_mut(lines_entity) {
                    lines_component.lines = all_hex_lines;
                    lines_component.mark_dirty();
                }
                self.lines_entity = Some(lines_entity);

                let range_lines_entity = world.spawn_entities(
                    LINES | VISIBILITY | LOCAL_TRANSFORM | GLOBAL_TRANSFORM | LOCAL_TRANSFORM_DIRTY,
                    1,
                )[0];
                if let Some(lines_component) = world.get_lines_mut(range_lines_entity) {
                    lines_component.lines = Vec::new();
                    lines_component.mark_dirty();
                }
                if let Some(visibility) = world.get_visibility_mut(range_lines_entity) {
                    visibility.visible = false;
                }
                self.range_lines_entity = Some(range_lines_entity);

                let initial_unit_coords = [
                    HexCoord::new(0, 0),
                    HexCoord::new(2, 1),
                    HexCoord::new(-2, -1),
                ];

                for coord in initial_unit_coords {
                    let position = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);
                    let unit_entity = spawn_unit(world, position);
                    self.units.push(Unit {
                        entity: unit_entity,
                        hex_coord: coord,
                        movement_range: 3,
                    });
                }
            }
            (Err(error), _) => {
                log::error!("Failed to load GLTF: {}", error);
            }
            (_, Err(error)) => {
                log::error!("Failed to load grass GLTF: {}", error);
            }
        }

        let camera_entity = spawn_pan_orbit_camera(
            world,
            nalgebra_glm::vec3(0.0, 0.0, 0.0),
            4000.0,
            0.3,
            0.8,
            "Hex War Camera".to_string(),
        );

        world.resources.active_camera = Some(camera_entity);

        let fps_props = TextProperties {
            font_size: 24.0,
            color: nalgebra_glm::vec4(1.0, 1.0, 1.0, 1.0),
            alignment: TextAlignment::Right,
            outline_width: 0.02,
            outline_color: nalgebra_glm::vec4(0.0, 0.0, 0.0, 1.0),
            ..Default::default()
        };
        self.fps_text_entity = Some(spawn_hud_text_with_properties(
            world,
            "FPS: 0",
            HudAnchor::TopRight,
            nalgebra_glm::vec2(-10.0, 10.0),
            fps_props,
        ));
    }

    fn run_systems(&mut self, world: &mut World) {
        pan_orbit_camera_system(world);

        let delta_time = world.resources.window.timing.delta_time;
        self.update_unit_movements(world, delta_time);

        let fps = world.resources.window.timing.frames_per_second;
        if let Some(fps_entity) = self.fps_text_entity {
            let text_index = world.get_hud_text(fps_entity).map(|t| t.text_index);
            if let Some(text_index) = text_index {
                world
                    .resources
                    .text_cache
                    .set_text(text_index, format!("FPS: {:.0}", fps));
                if let Some(hud_text) = world.get_hud_text_mut(fps_entity) {
                    hud_text.dirty = true;
                }
            }
        }

        if self.needs_regeneration && !self.tile_prefabs.is_empty() {
            self.needs_regeneration = false;

            for group in self.instanced_tile_groups.drain(..) {
                world.queue_command(WorldCommand::DespawnRecursive { entity: group.entity });
            }
            if let Some(lines_entity) = self.lines_entity.take() {
                world.queue_command(WorldCommand::DespawnRecursive {
                    entity: lines_entity,
                });
            }
            if let Some(range_lines_entity) = self.range_lines_entity.take() {
                world.queue_command(WorldCommand::DespawnRecursive {
                    entity: range_lines_entity,
                });
            }
            for unit in self.units.drain(..) {
                world.queue_command(WorldCommand::DespawnRecursive { entity: unit.entity });
            }
            self.coord_to_tile_type.clear();
            self.hovered_tile = None;
            self.selected_unit = None;
            self.selection_state = SelectionState::None;
            self.valid_move_tiles.clear();

            self.rng_seed = rand::rng().random();
            let river_tiles = generate_river_set(&self.params, self.rng_seed);
            let mut all_hex_lines: Vec<Line> = Vec::new();
            let mut tile_positions: Vec<(HexCoord, TileType)> = Vec::new();

            for row in -self.params.grid_radius..=self.params.grid_radius {
                for column in -self.params.grid_radius..=self.params.grid_radius {
                    let tile_type =
                        generate_tile_type(column, row, &self.params, self.rng_seed, &river_tiles);
                    let hex_coord = HexCoord::new(column, row);
                    tile_positions.push((hex_coord, tile_type));
                    self.coord_to_tile_type.insert(hex_coord, tile_type);

                    let position =
                        hex_to_world_position(column, row, self.hex_width, self.hex_depth);
                    let hex_lines =
                        generate_hex_outline(position, self.hex_width, self.hex_depth, 5.0);
                    all_hex_lines.extend(hex_lines);
                }
            }

            self.instanced_tile_groups = create_instanced_tiles(
                world,
                &self.tile_prefabs,
                &tile_positions,
                self.hex_width,
                self.hex_depth,
            );

            let lines_entity = world.spawn_entities(
                LINES | VISIBILITY | LOCAL_TRANSFORM | GLOBAL_TRANSFORM | LOCAL_TRANSFORM_DIRTY,
                1,
            )[0];
            if let Some(lines_component) = world.get_lines_mut(lines_entity) {
                lines_component.lines = all_hex_lines;
                lines_component.mark_dirty();
            }
            self.lines_entity = Some(lines_entity);

            let range_lines_entity = world.spawn_entities(
                LINES | VISIBILITY | LOCAL_TRANSFORM | GLOBAL_TRANSFORM | LOCAL_TRANSFORM_DIRTY,
                1,
            )[0];
            if let Some(lines_component) = world.get_lines_mut(range_lines_entity) {
                lines_component.lines = Vec::new();
                lines_component.mark_dirty();
            }
            if let Some(visibility) = world.get_visibility_mut(range_lines_entity) {
                visibility.visible = false;
            }
            self.range_lines_entity = Some(range_lines_entity);

            let initial_unit_coords = [
                HexCoord::new(0, 0),
                HexCoord::new(2, 1),
                HexCoord::new(-2, -1),
            ];

            for coord in initial_unit_coords {
                let position = hex_to_world_position(coord.column, coord.row, self.hex_width, self.hex_depth);
                let unit_entity = spawn_unit(world, position);
                self.units.push(Unit {
                    entity: unit_entity,
                    hex_coord: coord,
                    movement_range: 3,
                });
            }
        }

        let mouse = &world.resources.input.mouse;
        let mouse_pos = mouse.position;
        let left_clicked = mouse.state.contains(MouseState::LEFT_JUST_PRESSED);
        let right_clicked = mouse.state.contains(MouseState::RIGHT_JUST_PRESSED);

        let picking_results = pick_entities(world, mouse_pos, PickingOptions::default());

        let mut closest_unit: Option<Entity> = None;
        for result in &picking_results {
            for unit in &self.units {
                if unit.entity == result.entity {
                    closest_unit = Some(unit.entity);
                    break;
                }
            }
            if closest_unit.is_some() {
                break;
            }
        }

        let hovered_tile_coord: Option<HexCoord> =
            if let Some(ray) = PickingRay::from_screen_position(world, mouse_pos) {
                if let Some(hit_point) = ray.intersect_ground_plane(0.0) {
                    let coord = world_to_hex(hit_point.x, hit_point.z, self.hex_width, self.hex_depth);
                    if self.coord_to_tile_type.contains_key(&coord) {
                        Some(coord)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

        if let Some(prev_hovered) = self.hovered_unit
            && Some(prev_hovered) != closest_unit
            && self.selected_unit != Some(prev_hovered)
            && let Some(material) = world.get_material_mut(prev_hovered)
        {
            material.base_color = [0.2, 0.6, 1.0, 1.0];
        }
        self.hovered_unit = closest_unit;

        if let Some(hovered_unit) = self.hovered_unit
            && self.selected_unit != Some(hovered_unit)
            && let Some(material) = world.get_material_mut(hovered_unit)
        {
            material.base_color = [0.5, 0.8, 1.0, 1.0];
        }

        self.hovered_tile = hovered_tile_coord;

        if right_clicked {
            self.clear_selection(world);
        }

        if left_clicked {
            match self.selection_state {
                SelectionState::None => {
                    if let Some(clicked_unit) = closest_unit {
                        self.select_unit(world, clicked_unit);
                    }
                }
                SelectionState::UnitSelected(selected_entity) => {
                    let clicked_different_unit =
                        closest_unit.is_some() && closest_unit != Some(selected_entity);

                    if clicked_different_unit {
                        self.clear_selection(world);
                        self.select_unit(world, closest_unit.unwrap());
                    } else if let Some(coord) = hovered_tile_coord
                        && self.valid_move_tiles.contains(&coord)
                    {
                        self.move_unit_to(world, selected_entity, coord);
                    } else if closest_unit == Some(selected_entity) {
                        self.clear_selection(world);
                    }
                }
            }
        }

        if let Some(selected) = self.selected_unit
            && let Some(material) = world.get_material_mut(selected)
        {
            material.base_color = [1.0, 0.8, 0.2, 1.0];
        }
    }

    fn ui(&mut self, _world: &mut World, ui_context: &egui::Context) {
        egui::Window::new("Map Generation")
            .default_pos([10.0, 10.0])
            .show(ui_context, |ui| {
                ui.heading("Map Settings");
                ui.add(egui::Slider::new(&mut self.params.grid_radius, 5..=50).text("Map Size"));
                ui.add(
                    egui::Slider::new(&mut self.params.noise_scale, 0.02..=0.15)
                        .text("Region Scale"),
                );

                ui.separator();
                ui.heading("Coastline");
                ui.add(
                    egui::Slider::new(&mut self.params.coast_threshold, 0.2..=0.6)
                        .text("Sea Level"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.coast_falloff, 0.0..=0.3)
                        .text("Edge Falloff"),
                );

                ui.separator();
                ui.heading("Rivers & Lakes");
                ui.add(
                    egui::Slider::new(&mut self.params.lake_threshold, 0.4..=0.9)
                        .text("Lake Rarity"),
                );
                ui.add(egui::Slider::new(&mut self.params.river_width, 0..=4).text("River Width"));
                ui.add(
                    egui::Slider::new(&mut self.params.num_tributaries, 0..=8).text("Tributaries"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.meander_chance, 0..=80).text("River Curves"),
                );

                ui.separator();
                ui.heading("Biomes");
                ui.add(
                    egui::Slider::new(&mut self.params.desert_temp_threshold, 0.3..=0.8)
                        .text("Desert Heat"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.desert_moisture_threshold, 0.2..=0.7)
                        .text("Desert Dryness"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.forest_moisture_threshold, 0.3..=0.8)
                        .text("Forest Moisture"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.forest_elevation_threshold, 0.3..=0.7)
                        .text("Forest Elevation"),
                );

                ui.separator();
                if ui.button("Regenerate (R)").clicked() {
                    self.needs_regeneration = true;
                }
                ui.label("Press R key to regenerate");

                ui.separator();
                ui.heading("Stats");
                ui.label(format!("Units: {}", self.units.len()));
                ui.label(format!("Tile coords: {}", self.coord_to_tile_type.len()));
                ui.label(format!("Instanced groups: {}", self.instanced_tile_groups.len()));

                if let Some(selected) = self.selected_unit {
                    let unit_info = self.units.iter().find(|u| u.entity == selected);
                    if let Some(unit) = unit_info {
                        ui.label(format!(
                            "Selected unit at ({}, {})",
                            unit.hex_coord.column, unit.hex_coord.row
                        ));
                        ui.label(format!("Movement range: {}", unit.movement_range));
                        ui.label(format!("Valid moves: {}", self.valid_move_tiles.len()));
                    }
                } else {
                    ui.label("No unit selected");
                }

                ui.separator();
                ui.heading("Controls");
                ui.label("Left-click: Select unit / Move unit");
                ui.label("Right-click: Deselect");
            });
    }

    fn on_keyboard_input(&mut self, _world: &mut World, key: KeyCode, state: KeyState) {
        if state == KeyState::Pressed && key == KeyCode::KeyR {
            self.needs_regeneration = true;
        }
    }
}
