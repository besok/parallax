pub mod pos2d;

use crate::vis2d::pos2d::{Pos2d, Pos2dElems};
use bevy::asset::Assets;
use bevy::color::{Color, Srgba};
use bevy::prelude::{
    Camera2d, Circle, ColorMaterial, Commands, Component, Mesh, Mesh2d, MeshMaterial2d, Node,
    PositionType, Rectangle, ResMut, Text, Transform, Val, default,
};
use std::collections::HashMap;

#[derive(Component)]
struct Position(usize, usize);

pub fn setup_floor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    actor_pos: bevy::prelude::Res<Pos2dElems>,
) {
    commands.spawn(Camera2d);

    let width = 35.0;
    let height = 35.0;
    let gap = 3.0;
    let offset_x = -width * 10.0;
    let offset_y = -height * 10.0;

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(width * 21.0, height * 21.0))),
        MeshMaterial2d(materials.add(Color::Srgba(Srgba::BLACK))),
        Transform::from_xyz(0.0, 0.0, -0.1),
    ));
    let actor_map: HashMap<_, _> = actor_pos
        .items
        .iter()
        .map(|p @ Pos2d { x, y, .. }| ((x, y), p))
        .collect();

    for x in 0..20 {
        for y in 0..20 {
            let cell_x = offset_x + (x as f32) * (width + gap);
            let cell_y = offset_y + (y as f32) * (height + gap);

            let is_machine = actor_map.contains_key(&(&x, &y));

            // Floor tile (lighter for walkways, darker for machine areas)
            let floor_color = if is_machine {
                Color::srgb(0.5, 0.5, 0.5)
            } else {
                Color::srgb(0.3, 0.3, 0.32)
            };

            // Spawn floor tile
            commands.spawn((
                Mesh2d(meshes.add(Rectangle::new(width, height))),
                MeshMaterial2d(materials.add(floor_color)),
                Transform::from_xyz(cell_x, cell_y, 0.0),
                Position(x, y),
            ));

            // If this is a machine cell, add a machine representation
            if is_machine {
                let pos = actor_map.get(&(&x, &y)).unwrap();

                // Main machine body
                commands.spawn((
                    Mesh2d(meshes.add(Rectangle::new(width * 0.85, height * 0.85))),
                    MeshMaterial2d(materials.add(pos.color)),
                    Transform::from_xyz(cell_x, cell_y, 0.1),
                    Position(x, y),
                ));

                // Control panel (small rectangle at the top)
                commands.spawn((
                    Mesh2d(meshes.add(Rectangle::new(width * 0.4, height * 0.15))),
                    MeshMaterial2d(materials.add(Color::Srgba(Srgba::WHITE))),
                    Transform::from_xyz(cell_x, cell_y + height * 0.25, 0.2),
                    Position(x, y),
                ));

                // Indicator light (small circle in the control panel)
                commands.spawn((
                    Mesh2d(meshes.add(Circle::new(width * 0.05))),
                    MeshMaterial2d(materials.add(Color::Srgba(Srgba::GREEN))),
                    Transform::from_xyz(cell_x, cell_y + height * 0.25, 0.3),
                    Position(x, y),
                ));
            }
        }
    }

    commands.spawn((
        Text::new("Factory Shopfloor Simulation"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}
