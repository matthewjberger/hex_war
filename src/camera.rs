use crate::hex::hex_to_world_position;
use nightshade::prelude::*;

pub struct CameraBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_z: f32,
    pub max_z: f32,
}

pub fn calculate_camera_bounds(
    hex_width: f32,
    hex_depth: f32,
    map_width: i32,
    map_height: i32,
) -> CameraBounds {
    let min_pos = hex_to_world_position(0, 0, hex_width, hex_depth);
    let max_pos = hex_to_world_position(map_width - 1, map_height - 1, hex_width, hex_depth);

    let padding_x = hex_width * 2.0;
    let padding_z = hex_depth * 2.0;

    CameraBounds {
        min_x: min_pos.x - padding_x,
        max_x: max_pos.x + padding_x,
        min_z: min_pos.z - padding_z,
        max_z: max_pos.z + padding_z,
    }
}

pub fn clamp_camera_to_bounds(world: &mut World, bounds: &CameraBounds) {
    let Some(camera_entity) = world.resources.active_camera else {
        return;
    };

    let Some(pan_orbit) = world.get_pan_orbit_camera_mut(camera_entity) else {
        return;
    };

    pan_orbit.target_focus.x = pan_orbit.target_focus.x.clamp(bounds.min_x, bounds.max_x);
    pan_orbit.target_focus.z = pan_orbit.target_focus.z.clamp(bounds.min_z, bounds.max_z);
}

pub fn world_to_screen(world: &World, world_pos: Vec3) -> Option<Vec2> {
    let camera_entity = world.resources.active_camera?;
    let camera = world.get_camera(camera_entity)?;
    let global_transform = world.get_global_transform(camera_entity)?;

    let window = &world.resources.window;
    let (viewport_width, viewport_height) = window.cached_viewport_size?;

    let view_matrix = global_transform.0.try_inverse()?;
    let aspect_ratio = viewport_width as f32 / viewport_height as f32;
    let projection_matrix = camera.projection.matrix_with_aspect(aspect_ratio);

    let clip_pos =
        projection_matrix * view_matrix * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);

    if clip_pos.w <= 0.0 {
        return None;
    }

    let ndc = clip_pos.xyz() / clip_pos.w;

    let screen_x = (ndc.x + 1.0) * 0.5 * viewport_width as f32;
    let screen_y = (1.0 - ndc.y) * 0.5 * viewport_height as f32;

    Some(Vec2::new(screen_x, screen_y))
}

pub fn reset_camera_to_map(
    world: &mut World,
    hex_width: f32,
    hex_depth: f32,
    map_width: i32,
    map_height: i32,
) {
    let Some(camera_entity) = world.resources.active_camera else {
        return;
    };

    let world_width = map_width as f32 * hex_width * 0.75;
    let world_height = map_height as f32 * hex_depth;

    let center_column = (map_width - 1) / 2;
    let center_row = (map_height - 1) / 2;
    let center_pos = hex_to_world_position(center_column, center_row, hex_width, hex_depth);

    let y_fov_rad = if let Some(camera) = world.get_camera(camera_entity) {
        match &camera.projection {
            Projection::Perspective(persp) => persp.y_fov_rad,
            Projection::Orthographic(_) => std::f32::consts::FRAC_PI_4,
        }
    } else {
        std::f32::consts::FRAC_PI_4
    };

    let (viewport_width, viewport_height) = world
        .resources
        .window
        .cached_viewport_size
        .unwrap_or((1920, 1080));
    let aspect_ratio = viewport_width as f32 / viewport_height as f32;

    let half_fov_tan = (y_fov_rad / 2.0).tan();
    let radius_for_height = (world_height / 2.0) / half_fov_tan;
    let radius_for_width = (world_width / 2.0) / (half_fov_tan * aspect_ratio);
    let radius = radius_for_height.max(radius_for_width) * 1.1;

    let Some(pan_orbit) = world.get_pan_orbit_camera_mut(camera_entity) else {
        return;
    };

    pan_orbit.target_focus = nalgebra_glm::vec3(center_pos.x, 0.0, center_pos.z);
    pan_orbit.target_radius = radius;
    pan_orbit.target_yaw = 0.0;
    pan_orbit.target_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
}
