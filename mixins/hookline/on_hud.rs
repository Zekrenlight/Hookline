fn hookline_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::hookline as hook;
    let _ = corners;
    if !game.native_on(hook::MOD_ID) || !game.native_window() {
        return;
    }
    let Some(line) = hook::overlay_line() else {
        return;
    };
    game.native_text(batch, &line, [8.0, 8.0], 0xFFE8DCC8, true);
}
