fn hookline_panel_key(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    key: &str,
    modifiers: u32,
    canceled: &mut bool,
) {
    let _ = modifiers;
    if crate::game::hookline_panel::key(game, screen, key) {
        *canceled = true;
    }
}
