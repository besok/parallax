use bevy::prelude::*;
use parallax_kernel::vis2d::pos2d::{Pos2d, Pos2dElems};
use parallax_kernel::vis2d::setup_floor;

fn main() {
    let mut app = App::new();
    let pos_elems = Pos2dElems::new(vec![
        Pos2d::red_with_label(1, 1, "W01"),
        Pos2d::red_with_label(1, 2, "W02"),
        Pos2d::red_with_label(1, 2, "W04"),
        Pos2d::red_with_label(2, 3, "W05"),
        Pos2d::green_with_label(3, 3, "Cutting Center"),
        Pos2d::green(4, 3),
    ]);

    app.insert_resource(pos_elems)
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_floor);
    app.run();
}
