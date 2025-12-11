use crate::ecs::TileType;
use crate::hex::{HexCoord, hex_to_world_position};
use nightshade::ecs::prefab::Prefab;
use nightshade::prelude::*;
use std::collections::HashMap;

pub struct InstancedTileGroup {
    pub entity: Entity,
    pub coord_to_instance: HashMap<HexCoord, usize>,
}

type MeshInstanceKey = (String, u64);
type MeshInstanceValue = (Material, Vec<(HexCoord, InstanceTransform)>);

struct ExtractedMesh {
    mesh_name: String,
    material: Material,
    local_transform: LocalTransform,
}

fn extract_meshes_from_prefab(prefab: &Prefab) -> Vec<ExtractedMesh> {
    use nightshade::ecs::prefab::PrefabNode;

    fn extract_meshes_from_node(
        node: &PrefabNode,
        parent_transform: LocalTransform,
        meshes: &mut Vec<ExtractedMesh>,
    ) {
        let combined_translation = parent_transform.translation
            + nalgebra_glm::quat_rotate_vec3(
                &parent_transform.rotation,
                &nalgebra_glm::vec3(
                    node.local_transform.translation.x * parent_transform.scale.x,
                    node.local_transform.translation.y * parent_transform.scale.y,
                    node.local_transform.translation.z * parent_transform.scale.z,
                ),
            );
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

    let mut meshes = Vec::new();
    for node in &prefab.root_nodes {
        extract_meshes_from_node(node, LocalTransform::default(), &mut meshes);
    }
    meshes
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

pub fn create_instanced_tiles(
    world: &mut World,
    tile_prefabs: &HashMap<TileType, Prefab>,
    tile_positions: &[(HexCoord, TileType)],
    hex_width: f32,
    hex_depth: f32,
) -> Vec<InstancedTileGroup> {
    use nightshade::ecs::world::spawn_instanced_mesh_with_material;

    let prefab_meshes: HashMap<TileType, Vec<ExtractedMesh>> = tile_prefabs
        .iter()
        .map(|(tile_type, prefab)| (*tile_type, extract_meshes_from_prefab(prefab)))
        .collect();

    let mut mesh_instances: HashMap<MeshInstanceKey, MeshInstanceValue> = HashMap::new();

    for (coord, tile_type) in tile_positions {
        let Some(extracted_meshes) = prefab_meshes.get(tile_type) else {
            continue;
        };

        let tile_world_pos = hex_to_world_position(coord.column, coord.row, hex_width, hex_depth);

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

            let entry = mesh_instances
                .entry(key)
                .or_insert_with(|| (extracted.material.clone(), Vec::new()));
            entry.1.push((*coord, instance));
        }
    }

    let mut instanced_groups = Vec::new();

    for ((mesh_name, _mat_hash), (material, coord_instances)) in &mesh_instances {
        if coord_instances.is_empty() {
            continue;
        }

        let mut coord_to_instance = HashMap::new();
        let mut instances = Vec::with_capacity(coord_instances.len());

        for (index, (coord, transform)) in coord_instances.iter().enumerate() {
            coord_to_instance.insert(*coord, index);
            instances.push(*transform);
        }

        let entity =
            spawn_instanced_mesh_with_material(world, mesh_name, instances, material.clone());
        instanced_groups.push(InstancedTileGroup {
            entity,
            coord_to_instance,
        });
    }

    instanced_groups
}
