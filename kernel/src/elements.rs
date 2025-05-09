use bevy::prelude::Component;
use tokio::sync::mpsc::Sender;

#[derive(Component, Debug, Clone, PartialEq)]
pub struct Id(String);

#[derive(Component)]
pub struct Cell;

#[derive(Component)]
pub struct Bay {
    id: Id,
    capacity: usize,
    items: Vec<(Id, usize)>,
}
