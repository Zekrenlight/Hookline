fn hookline_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::hookline as hook;
    crate::game::hookline_panel::close_if_asked(game);
    if !game.native_on(hook::MOD_ID) || !game.native_window() {
        return;
    }
    hook::ensure_ready(game);
    hook::revise(hook::tick_flash);
    if game.player.is_none() {
        return;
    }
    hook::drive_tick(game);
}
