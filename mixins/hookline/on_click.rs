fn hookline_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::hookline as hook;
    if mod_id != hook::MOD_ID {
        return;
    }
    hook::ensure_ready(game);
    if button == 0 {
        crate::game::hookline_panel::open(game);
        return;
    }
    let Some(&(key, _)) = hook::HABITS.get(button - 1) else {
        return;
    };
    hook::revise(|s| {
        hook::flip_habit(s, key);
        hook::write_prefs_and_native(game, s);
    });
}
