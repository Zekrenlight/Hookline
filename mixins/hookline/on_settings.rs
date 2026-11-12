fn hookline_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::hookline as hook;
    if mod_id != hook::MOD_ID {
        return;
    }
    hook::ensure_ready(game);
    buttons.push("Open panel".into());
    hook::revise(|s| {
        for (key, label) in hook::HABITS {
            let on = hook::habit_on(s, key);
            buttons.push(format!("{label}: {}", if on { "On" } else { "Off" }));
        }
    });
}
