//! Development tools for the game. This plugin is only enabled in dev builds.

use avian3d::prelude::{PhysicsDebugPlugin, PhysicsGizmos};
use bevy::{
    dev_tools::states::log_transitions, input::common_conditions::input_just_pressed, prelude::*,
    window::CursorOptions,
};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use crate::screens::{Screen, set_cursor_grab};

pub(super) fn plugin(app: &mut App) {
    // Log `Screen` state transitions.
    app.add_plugins((
        EguiPlugin::default(),
        WorldInspectorPlugin::default().run_if(|options: Res<UiDebugOptions>| options.enabled),
        PhysicsDebugPlugin,
    ));
    app.add_systems(Startup, |mut store: ResMut<GizmoConfigStore>| {
        store.config_mut::<PhysicsGizmos>().0.enabled = false;
    });
    app.add_systems(Update, log_transitions::<Screen>);

    // Toggle the debug overlay for UI.
    app.add_systems(
        Update,
        toggle_debug_ui.run_if(input_just_pressed(TOGGLE_KEY)),
    );
}

const TOGGLE_KEY: KeyCode = KeyCode::Backquote;

fn toggle_debug_ui(
    mut options: ResMut<UiDebugOptions>,
    mut store: ResMut<GizmoConfigStore>,
    // mut cursor_options: Single<&mut CursorOptions>,
) {
    options.toggle();

    // set_cursor_grab(&mut cursor_options, !options.enabled);

    // physics debug
    let enabled = &mut store.config_mut::<PhysicsGizmos>().0.enabled;
    *enabled = !*enabled;
}
