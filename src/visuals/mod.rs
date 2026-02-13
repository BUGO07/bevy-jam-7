use bevy::app::App;

pub mod goop;

pub fn plugin(app: &mut App) {
    app.add_plugins(goop::plugin);
}
