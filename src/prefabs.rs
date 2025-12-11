use crate::ecs::TileType;
use nightshade::ecs::prefab::{
    GltfLoadResult, MeshCache, Prefab, PrefabNode, import_gltf_from_bytes,
};
use nightshade::prelude::*;
use std::collections::HashMap;

const HEXAGON_TILES_GLB: &[u8] = include_bytes!("../assets/hexagon_tiles.glb");
const GRASS_GLB: &[u8] = include_bytes!("../assets/grass.glb");

pub struct LoadedPrefabs {
    pub tile_prefabs: HashMap<TileType, Prefab>,
    pub hex_width: f32,
    pub hex_depth: f32,
}

pub fn load_tile_prefabs(world: &mut World) -> Option<LoadedPrefabs> {
    let tiles_result = import_gltf_from_bytes(HEXAGON_TILES_GLB);
    let grass_result = import_gltf_from_bytes(GRASS_GLB);

    match (tiles_result, grass_result) {
        (Ok(tiles), Ok(grass)) => {
            load_textures_and_meshes(world, &tiles);
            load_textures_and_meshes(world, &grass);

            let tile_prefabs = extract_tile_prefabs(&tiles, &grass, &world.resources.mesh_cache);

            if tile_prefabs.is_empty() {
                log::error!("No tile prefabs found!");
                return None;
            }

            let hex_width = tile_prefabs
                .values()
                .next()
                .and_then(|prefab| calculate_prefab_bounds(prefab, &world.resources.mesh_cache))
                .map(|(min_x, max_x, _, _)| max_x - min_x)
                .unwrap_or(173.205);
            let hex_depth = tile_prefabs
                .values()
                .next()
                .and_then(|prefab| calculate_prefab_bounds(prefab, &world.resources.mesh_cache))
                .map(|(_, _, min_z, max_z)| max_z - min_z)
                .unwrap_or(200.0);

            Some(LoadedPrefabs {
                tile_prefabs,
                hex_width,
                hex_depth,
            })
        }
        (Err(error), _) => {
            log::error!("Failed to load GLTF: {}", error);
            None
        }
        (_, Err(error)) => {
            log::error!("Failed to load grass GLTF: {}", error);
            None
        }
    }
}

fn load_textures_and_meshes(world: &mut World, result: &GltfLoadResult) {
    for (name, (rgba_data, width, height)) in &result.textures {
        world.queue_command(WorldCommand::LoadTexture {
            name: name.clone(),
            rgba_data: rgba_data.clone(),
            width: *width,
            height: *height,
        });
    }

    for (name, mesh) in &result.meshes {
        world
            .resources
            .mesh_cache
            .insert(name.clone(), mesh.clone());
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

fn calculate_prefab_bounds(
    prefab: &Prefab,
    mesh_cache: &MeshCache,
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
        mesh_cache: &MeshCache,
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
            find_all_mesh_bounds(child, current_scale, min_x, max_x, min_z, max_z, mesh_cache);
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

const TILE_TYPE_PREFAB_NAMES: [(TileType, &str); 5] = [
    (TileType::Sea, "sea"),
    (TileType::Land, "grass"),
    (TileType::Forest, "normal forest"),
    (TileType::City, "startingTile"),
    (TileType::Port, "tile animalFarm"),
];

fn extract_tile_prefabs(
    tiles_result: &GltfLoadResult,
    grass_result: &GltfLoadResult,
    mesh_cache: &MeshCache,
) -> HashMap<TileType, Prefab> {
    let mut tile_prefabs: HashMap<TileType, Prefab> = HashMap::new();

    let reference_prefab = tiles_result
        .prefabs
        .iter()
        .find_map(|prefab| find_node_by_name(&prefab.root_nodes, "normal forest"));

    let (hex_width, hex_depth) = if let Some(node) = reference_prefab {
        let mut zeroed_node = node.clone();
        zeroed_node.local_transform.translation = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        let temp_prefab = Prefab {
            name: "reference".to_string(),
            root_nodes: vec![zeroed_node],
        };
        calculate_hex_dimensions_from_prefab(&temp_prefab, mesh_cache)
    } else {
        (173.205, 200.0)
    };

    for (tile_type, prefab_name) in TILE_TYPE_PREFAB_NAMES {
        if prefab_name == "grass" {
            if let Some(grass_prefab) = grass_result.prefabs.first() {
                let scaled_grass =
                    scale_grass_prefab(grass_prefab, hex_width, hex_depth, mesh_cache);
                tile_prefabs.insert(tile_type, scaled_grass);
            }
        } else if let Some(node) = tiles_result
            .prefabs
            .iter()
            .find_map(|prefab| find_node_by_name(&prefab.root_nodes, prefab_name))
        {
            let mut zeroed_node = node.clone();
            zeroed_node.local_transform.translation = nalgebra_glm::vec3(0.0, 0.0, 0.0);
            tile_prefabs.insert(
                tile_type,
                Prefab {
                    name: prefab_name.to_string(),
                    root_nodes: vec![zeroed_node],
                },
            );
        }
    }

    if let Some(city_prefab) = tile_prefabs.get(&TileType::City).cloned() {
        tile_prefabs.insert(TileType::Capital, city_prefab);
    }

    tile_prefabs
}

fn calculate_hex_dimensions_from_prefab(prefab: &Prefab, mesh_cache: &MeshCache) -> (f32, f32) {
    if let Some((min_x, max_x, min_z, max_z)) = calculate_prefab_bounds(prefab, mesh_cache) {
        let width = max_x - min_x;
        let depth = max_z - min_z;
        log::info!(
            "Tile dimensions from hexagon_tiles: width={}, depth={}",
            width,
            depth
        );
        (width, depth)
    } else {
        (173.205, 200.0)
    }
}

fn scale_grass_prefab(
    grass_prefab: &Prefab,
    hex_width: f32,
    hex_depth: f32,
    mesh_cache: &MeshCache,
) -> Prefab {
    let mut grass_clone = grass_prefab.clone();
    grass_clone.name = "grass".to_string();

    if let Some((min_x, max_x, min_z, max_z)) = calculate_prefab_bounds(&grass_clone, mesh_cache) {
        let grass_width = max_x - min_x;
        let grass_depth = max_z - min_z;

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

            for root_node in &mut grass_clone.root_nodes {
                root_node.local_transform.translation = nalgebra_glm::vec3(0.0, 0.0, 0.0);
                root_node.local_transform.scale = nalgebra_glm::vec3(
                    root_node.local_transform.scale.x * scale,
                    root_node.local_transform.scale.y * scale,
                    root_node.local_transform.scale.z * scale,
                );
            }
        }
    } else {
        for root_node in &mut grass_clone.root_nodes {
            root_node.local_transform.translation = nalgebra_glm::vec3(0.0, 0.0, 0.0);
        }
    }

    grass_clone
}
