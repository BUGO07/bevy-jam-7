//! The death menu.

use bevy::{prelude::*, window::CursorOptions};

use crate::{
    menus::Menu,
    screens::{Screen, set_cursor_grab},
    theme::widget,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Menu::Death), spawn_death_menu);
}

fn spawn_death_menu(mut commands: Commands, mut cursor_options: Single<&mut CursorOptions>) {
    set_cursor_grab(&mut cursor_options, false);

    commands.spawn((
        widget::ui_root("Death Menu"),
        GlobalZIndex(2),
        DespawnOnExit(Menu::Death),
        children![
            widget::header("You Died"),
            widget::button("Respawn", respawn),
            widget::button("Quit to title", quit_to_title),
        ],
    ));
}

fn respawn(
    _: On<Pointer<Click>>,
    mut next_menu: ResMut<NextState<Menu>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    set_cursor_grab(&mut cursor_options, true);

    next_menu.set(Menu::None);
}

fn quit_to_title(_: On<Pointer<Click>>, mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Title);
}
