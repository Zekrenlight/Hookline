fn hookline_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use crate::game::hookline as hook;
    if !game.native_on(hook::MOD_ID) {
        return;
    }
    hook::ensure_ready(game);
    hook::handle_command(game, command, ran);
}
