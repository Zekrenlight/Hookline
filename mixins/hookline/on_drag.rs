// Held-button movement: moves the Loot/Enchants scrollbar thumb.
fn hookline_drag(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if crate::game::hookline_panel::drag(game, screen, button) {
        *canceled = true;
    }
}
