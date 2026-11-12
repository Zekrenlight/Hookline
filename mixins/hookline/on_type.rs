fn hookline_char(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    c: char,
    modifiers: u32,
    canceled: &mut bool,
) {
    let _ = modifiers;
    if crate::game::hookline_panel::typed(game, screen, c) {
        *canceled = true;
    }
}
