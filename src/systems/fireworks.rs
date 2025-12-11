use crate::ecs::{Faction, TileType, faction_color};
use nightshade::ecs::particles::components::{ColorGradient, EmitterShape, EmitterType};
use nightshade::prelude::*;

const SCALE: f32 = 30.0;

pub struct FireworkShell {
    pub entity: Entity,
    pub position: Vec3,
    pub velocity: Vec3,
    pub fuse_time: f32,
    pub color: Vec3,
    pub particle_count: u32,
    pub is_capital: bool,
}

fn scale_emitter(emitter: &mut ParticleEmitter) {
    emitter.initial_velocity_min *= SCALE;
    emitter.initial_velocity_max *= SCALE;
    emitter.size_start *= SCALE;
    emitter.size_end *= SCALE;
    emitter.gravity *= SCALE;
    if let EmitterShape::Sphere { ref mut radius } = emitter.shape {
        *radius *= SCALE;
    }
}

fn create_shell_trail(position: Vec3) -> ParticleEmitter {
    ParticleEmitter {
        emitter_type: EmitterType::Firework,
        shape: EmitterShape::Point,
        position,
        direction: nalgebra_glm::vec3(0.0, -1.0, 0.0),
        spawn_rate: 150.0,
        burst_count: 0,
        particle_lifetime_min: 0.3,
        particle_lifetime_max: 0.6,
        initial_velocity_min: 5.0 * SCALE,
        initial_velocity_max: 15.0 * SCALE,
        velocity_spread: 0.4,
        gravity: nalgebra_glm::vec3(0.0, -2.0 * SCALE, 0.0),
        drag: 0.1,
        size_start: 0.2 * SCALE,
        size_end: 0.05 * SCALE,
        color_gradient: ColorGradient::firework_trail(),
        emissive_strength: 8.0,
        enabled: true,
        accumulated_spawn: 0.0,
        one_shot: false,
        has_fired: false,
        turbulence_strength: 0.3,
        turbulence_frequency: 2.0,
    }
}

pub fn spawn_capture_firework(
    shells: &mut Vec<FireworkShell>,
    world: &mut World,
    position: Vec3,
    tile_type: TileType,
    faction: Faction,
) {
    let color = faction_color(faction);
    let color_vec = nalgebra_glm::vec3(color[0], color[1], color[2]);

    let launch_pos = nalgebra_glm::vec3(position.x, position.y, position.z);

    let particle_count = match tile_type {
        TileType::Capital => 1200,
        TileType::City => 800,
        TileType::Port => 500,
        _ => 300,
    };

    let shell_count = match tile_type {
        TileType::Capital => 8,
        TileType::City => 1,
        TileType::Port => 1,
        _ => 1,
    };

    for shell_index in 0..shell_count {
        let mut rng = rand::rng();
        let x_spread = rng.random_range(-200.0..200.0);
        let z_spread = rng.random_range(-200.0..200.0);
        let shell_launch_pos = nalgebra_glm::vec3(
            launch_pos.x + x_spread,
            launch_pos.y,
            launch_pos.z + z_spread,
        );

        let velocity = nalgebra_glm::vec3(
            rng.random_range(-50.0..50.0),
            rng.random_range(400.0..600.0),
            rng.random_range(-50.0..50.0),
        );

        let target_height = rng.random_range(400.0..700.0);
        let fuse_time = target_height / velocity.y + (shell_index as f32) * 0.2;

        let entity = world.spawn_entities(nightshade::ecs::PARTICLE_EMITTER, 1)[0];
        let trail_emitter = create_shell_trail(shell_launch_pos);
        world.set_particle_emitter(entity, trail_emitter);

        shells.push(FireworkShell {
            entity,
            position: shell_launch_pos,
            velocity,
            fuse_time,
            color: color_vec,
            particle_count,
            is_capital: tile_type == TileType::Capital,
        });
    }
}

pub fn update_firework_shells(shells: &mut Vec<FireworkShell>, world: &mut World, delta_time: f32) {
    let mut explosions: Vec<(Vec3, Vec3, u32, bool, Entity)> = Vec::new();

    for shell in shells.iter_mut() {
        shell.fuse_time -= delta_time;
        shell.position += shell.velocity * delta_time;
        shell.velocity.y -= 200.0 * delta_time;

        if let Some(emitter) = world.get_particle_emitter_mut(shell.entity) {
            emitter.position = shell.position;
        }

        if shell.fuse_time <= 0.0 {
            explosions.push((
                shell.position,
                shell.color,
                shell.particle_count,
                shell.is_capital,
                shell.entity,
            ));
        }
    }

    for (pos, color, particle_count, is_capital, entity) in explosions {
        let flash_entity = world.spawn_entities(nightshade::ecs::PARTICLE_EMITTER, 1)[0];
        let mut flash_emitter = ParticleEmitter::flash_burst(pos);
        scale_emitter(&mut flash_emitter);
        world.set_particle_emitter(flash_entity, flash_emitter);

        let explosion_entity = world.spawn_entities(nightshade::ecs::PARTICLE_EMITTER, 1)[0];
        let mut emitter = ParticleEmitter::firework_explosion(pos, color, particle_count);
        scale_emitter(&mut emitter);
        world.set_particle_emitter(explosion_entity, emitter);

        let glitter_entity = world.spawn_entities(nightshade::ecs::PARTICLE_EMITTER, 1)[0];
        let glitter_count = particle_count / 2;
        let mut glitter_emitter = ParticleEmitter::firework_glitter(pos, glitter_count);
        scale_emitter(&mut glitter_emitter);
        world.set_particle_emitter(glitter_entity, glitter_emitter);

        if is_capital {
            for ring_index in 0..6 {
                let angle = (ring_index as f32) * std::f32::consts::TAU / 6.0;
                let ring_entity = world.spawn_entities(nightshade::ecs::PARTICLE_EMITTER, 1)[0];
                let mut ring_emitter = ParticleEmitter::firework_ring(pos, color, 400);
                ring_emitter.direction = nalgebra_glm::vec3(angle.sin(), 0.0, angle.cos());
                scale_emitter(&mut ring_emitter);
                world.set_particle_emitter(ring_entity, ring_emitter);
            }
        }

        if let Some(emitter) = world.get_particle_emitter_mut(entity) {
            emitter.enabled = false;
        }
    }

    shells.retain(|shell| shell.fuse_time > 0.0);
}
