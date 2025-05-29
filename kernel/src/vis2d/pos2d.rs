use bevy::color::{Color, Srgba};
use bevy::prelude::{Component, Resource};

#[derive(Component, Debug, Clone, PartialEq)]
pub struct Pos2d {
    pub x: usize,
    pub y: usize,
    pub color: Color,
    pub label: Option<String>,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Pos2dElems {
    pub items: Vec<Pos2d>,
}

impl Pos2dElems {
    pub fn new(items: Vec<Pos2d>) -> Self {
        Self { items }
    }

    pub fn empty() -> Self {
        Self::new(vec![])
    }
}

impl Pos2d {
    pub fn new(x: usize, y: usize, color: Color) -> Self {
        Self {
            x,
            y,
            color,
            label: None,
        }
    }

    pub fn new_with_label(x: usize, y: usize, color: Color, label: &str) -> Self {
        Self {
            x,
            y,
            color,
            label: Some(label.to_string()),
        }
    }

    pub fn blue(x: usize, y: usize) -> Self {
        Self::new(x, y, Color::Srgba(Srgba::BLUE))
    }
    pub fn green(x: usize, y: usize) -> Self {
        Self::new(x, y, Color::Srgba(Srgba::GREEN))
    }
    pub fn red(x: usize, y: usize) -> Self {
        Self::new(x, y, Color::Srgba(Srgba::RED))
    }

    pub fn blue_with_label(x: usize, y: usize, label: &str) -> Self {
        Self::new_with_label(x, y, Color::Srgba(Srgba::BLUE), label)
    }

    pub fn green_with_label(x: usize, y: usize, label: &str) -> Self {
        Self::new_with_label(x, y, Color::Srgba(Srgba::GREEN), label)
    }

    pub fn red_with_label(x: usize, y: usize, label: &str) -> Self {
        Self::new_with_label(x, y, Color::Srgba(Srgba::RED), label)
    }
}
