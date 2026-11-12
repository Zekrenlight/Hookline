// Mouse button let go: ends a scrollbar thumb drag on Hookline's panel.
fn hookline_release(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if crate::game::hookline_panel::release(game, screen, button) {
        *canceled = true;
    }
}
