fn hookline_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::hookline as hook;
    if !game.native_on(hook::MOD_ID) {
        return;
    }
    hook::ensure_ready(game);
    let toggle = game.native_mod_key(hook::MOD_ID, "toggle", crate::game::hookline_logic::persist::TOGGLE_KEY);
    let panel = game.native_mod_key(hook::MOD_ID, "panel", crate::game::hookline_logic::persist::PANEL_KEY);
    if key == toggle && hook::rising("toggle", pressed) {
        hook::flip_from_key(game);
    }
    if key == panel && hook::rising("panel", pressed) {
        if game.native_mod_screen() == Some(hook::MOD_ID) {
            game.native_mod_screen_close(hook::MOD_ID);
        } else if game.native_playing() {
            let _ = game.native_mod_screen_open(hook::MOD_ID);
        }
    }
}
