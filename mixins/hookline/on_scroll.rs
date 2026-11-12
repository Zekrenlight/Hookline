fn hookline_scroll(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    if crate::game::hookline_panel::wheel(game, screen, scroll) {
        *canceled = true;
    }
}
