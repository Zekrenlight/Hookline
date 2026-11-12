fn hookline_press(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    modifiers: u32,
    canceled: &mut bool,
) {
    let _ = modifiers;
    if crate::game::hookline_panel::click(game, screen, button) {
        *canceled = true;
    }
}
