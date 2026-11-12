// Hookline's tabbed card: Loot, Enchants, Behavior, Stats.

use super::hookline as hook;
use super::Game;
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use crate::game::hookline_logic as rules;
use rules::catalog::{label_of, CHARMS, CATCH_ROWS};
use rules::persist::cycle_charm_floor;
use rules::{CHARM_ANY, CHARM_TOSS};

const SHADE: u32 = 0xA0081014;
const CARD: u32 = 0xFF10262C;
const EDGE: u32 = 0xFF1E4A4E;
const TAB: u32 = 0xFF16333C;
const TAB_ON: u32 = 0xFF2A8F7A;
const KEEP: u32 = 0xFF2E6B4F;
const TOSS: u32 = 0xFF8B3A2A;
const CHIP: u32 = 0xFF2A3C44;
const TEXT: u32 = 0xFFE8DCC8;
const DIM: u32 = 0xFF8A9A94;
const ACCENT: u32 = 0xFFE0A14A;
const SEARCH: u32 = 0xFF0A1618;
const TABS: &[&str] = &["Loot", "Enchants", "Behavior", "Stats"];

pub fn open(game: &mut Game<'_>) {
    let _ = game.native_mod_screen_open(hook::MOD_ID);
}

fn ours(game: &Game<'_>, screen: &NativeScreenView<'_>) -> bool {
    game.native_on(hook::MOD_ID) && game.native_mod_screen() == Some(hook::MOD_ID) && screen.id == "mod"
}

fn hit(p: (f32, f32), r: [f32; 4]) -> bool {
    p.0 >= r[0] && p.0 < r[0] + r[2] && p.1 >= r[1] && p.1 < r[1] + r[3]
}

struct Frame {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    done: [f32; 4],
    tabs: [[f32; 4]; 4],
    body: [f32; 4],
    search: [f32; 4],
}

fn frame(screen: &NativeScreenView<'_>) -> Frame {
    let sw = screen.rect[2].max(1.0);
    let sh = screen.rect[3].max(1.0);
    let w = (sw * 0.62).clamp(280.0, 460.0);
    let h = (sh * 0.72).clamp(rules::listing::PANEL_H_MIN, rules::listing::PANEL_H_MAX);
    let x = ((sw - w) * 0.5).max(8.0);
    let y = ((sh - h) * 0.5).max(8.0);
    let done_w = 52.0;
    let done = [x + w - done_w - 10.0, y + 8.0, done_w, 16.0];
    let tab_y = y + 28.0;
    let tab_w = (w - 20.0) / 4.0;
    let mut tabs = [[0.0; 4]; 4];
    for i in 0..4 {
        tabs[i] = [x + 10.0 + tab_w * i as f32, tab_y, tab_w - 2.0, 18.0];
    }
    let body = [x + 10.0, tab_y + 24.0, w - 20.0, rules::listing::body_h(h)];
    let search = [body[0], body[1], body[2], 16.0];
    Frame { x, y, w, h, done, tabs, body, search }
}

fn fill(game: &Game<'_>, batch: &mut HudBatch, r: [f32; 4], c: u32) {
    game.native_fill(batch, r, c);
}

fn ink(game: &Game<'_>, batch: &mut HudBatch, text: &str, pos: [f32; 2], c: u32) {
    game.native_text(batch, text, pos, c, true);
}

fn ink_center(game: &Game<'_>, batch: &mut HudBatch, text: &str, cx: f32, y: f32, c: u32) {
    let w = game.native_text_width(text);
    ink(game, batch, text, [cx - w * 0.5, y], c);
}

fn chip(game: &Game<'_>, batch: &mut HudBatch, r: [f32; 4], label: &str, on: bool, hover: bool, color: u32) {
    let bg = if on { color } else if hover { 0xFF3A5158 } else { CHIP };
    fill(game, batch, r, bg);
    ink_center(game, batch, label, r[0] + r[2] * 0.5, r[1] + 3.0, TEXT);
}

fn loot_rows(query: &str) -> Vec<&'static str> {
    let q = query.trim().to_ascii_lowercase();
    CATCH_ROWS
        .iter()
        .copied()
        .filter(|id| q.is_empty() || id.contains(&q) || label_of(id).to_ascii_lowercase().contains(&q))
        .collect()
}

fn list_rect(body: [f32; 4], header: f32) -> [f32; 4] {
    let pad = rules::listing::BAR_W + rules::listing::BAR_GAP;
    let view = if (header - rules::listing::LOOT_HEADER).abs() < 0.01 {
        rules::listing::loot_view_h(body[3])
    } else if (header - rules::listing::CHARM_HEADER).abs() < 0.01 {
        rules::listing::charm_view_h(body[3])
    } else {
        (body[3] - header).max(0.0)
    };
    [
        body[0],
        body[1] + header,
        (body[2] - pad).max(0.0),
        view,
    ]
}

fn bar_track(list: [f32; 4]) -> [f32; 4] {
    [
        list[0] + list[2] + rules::listing::BAR_GAP,
        list[1],
        rules::listing::BAR_W,
        list[3],
    ]
}

fn row_box(list: [f32; 4], index: usize, scroll: f32) -> [f32; 4] {
    let y = list[1] + rules::listing::row_top(index, scroll);
    [list[0], y, list[2], rules::listing::ROW - 2.0]
}

fn apply_scroll(total: usize, view_h: f32, scroll: f32) -> (f32, usize, usize) {
    let scroll = rules::listing::clamp_scroll(scroll, total, view_h);
    let (start, end) = rules::listing::shown_rows(scroll, total, view_h);
    (scroll, start, end)
}

fn paint_bar(game: &Game<'_>, batch: &mut HudBatch, list: [f32; 4], scroll: f32, total: usize, hover: bool) {
    let track = bar_track(list);
    if track[3] <= 0.0 {
        return;
    }
    fill(game, batch, track, SEARCH);
    let Some(thumb) = rules::listing::thumb(scroll, total, list[3], track[3]) else {
        return;
    };
    let r = [track[0], track[1] + thumb.y, track[2], thumb.h];
    fill(game, batch, r, if hover { ACCENT } else { TAB_ON });
}

fn bar_hover(p: (f32, f32), list: [f32; 4], scroll: f32, total: usize) -> bool {
    let track = bar_track(list);
    let Some(thumb) = rules::listing::thumb(scroll, total, list[3], track[3]) else {
        return hit(p, track);
    };
    hit(p, [track[0], track[1] + thumb.y, track[2], thumb.h])
}

fn click_bar(p: (f32, f32), list: [f32; 4], total: usize, scroll: f32) -> bool {
    let track = bar_track(list);
    if !hit(p, track) {
        return false;
    }
    let local = p.1 - track[1];
    hook::revise(|s| {
        let view = list[3];
        if let Some(thumb) = rules::listing::thumb(scroll, total, view, track[3]) {
            if local >= thumb.y && local < thumb.y + thumb.h {
                s.bar_grab = Some(local - thumb.y);
                return;
            }
        }
        s.scroll = rules::listing::scroll_at_track(local, total, view, track[3]);
        if let Some(thumb) = rules::listing::thumb(s.scroll, total, view, track[3]) {
            s.bar_grab = Some((local - thumb.y).clamp(0.0, thumb.h));
        }
    });
    true
}

fn list_totals(ui: &Frame, tab: u8, query: &str) -> (usize, [f32; 4]) {
    match tab {
        0 => (loot_rows(query).len(), list_rect(ui.body, rules::listing::LOOT_HEADER)),
        1 => (rules::CHARMS.len(), list_rect(ui.body, rules::listing::CHARM_HEADER)),
        _ => (0, list_rect(ui.body, 0.0)),
    }
}

pub fn paint(game: &Game<'_>, batch: &mut HudBatch, screen: &NativeScreenView<'_>) {
    if !ours(game, screen) {
        return;
    }
    let ui = frame(screen);
    fill(game, batch, [0.0, 0.0, screen.rect[2], screen.rect[3]], SHADE);
    fill(game, batch, [ui.x, ui.y, ui.w, ui.h], CARD);
    fill(game, batch, [ui.x, ui.y, ui.w, 2.0], EDGE);
    ink(game, batch, "Hookline", [ui.x + 12.0, ui.y + 8.0], ACCENT);
    let p = (screen.mouse[0], screen.mouse[1]);
    chip(game, batch, ui.done, "Done", hit(p, ui.done), hit(p, ui.done), EDGE);
    let tab = hook::revise(|s| s.tab);
    for (i, label) in TABS.iter().enumerate() {
        let on = tab == i as u8;
        fill(game, batch, ui.tabs[i], if on { TAB_ON } else { TAB });
        ink_center(game, batch, label, ui.tabs[i][0] + ui.tabs[i][2] * 0.5, ui.tabs[i][1] + 4.0, if on { TEXT } else { DIM });
    }
    match tab {
        0 => paint_loot(game, batch, &ui, p),
        1 => paint_charms(game, batch, &ui, p),
        2 => paint_habits(game, batch, &ui, p),
        _ => paint_ledger(game, batch, &ui, p),
    }
}

fn paint_loot(game: &Game<'_>, batch: &mut HudBatch, ui: &Frame, p: (f32, f32)) {
    let (query, focus, scroll, toss, chime) = hook::revise(|s| {
        (s.query.clone(), s.query_focus, s.scroll, s.prefs.toss.clone(), s.prefs.chime.clone())
    });
    fill(game, batch, ui.search, SEARCH);
    let hint = if query.is_empty() {
        if focus { "_" } else { "Search catches..." }
    } else if focus {
        let mut t = query.clone();
        t.push('_');
        // leak avoided: use a binding
        let _ = t;
        "x"
    } else {
        query.as_str()
    };
    let shown = if query.is_empty() && !focus {
        "Search catches...".to_string()
    } else if focus {
        format!("{query}_")
    } else {
        query.clone()
    };
    ink(game, batch, &shown, [ui.search[0] + 4.0, ui.search[1] + 3.0], if query.is_empty() && !focus { DIM } else { TEXT });
    let _ = hint;
    let rows = loot_rows(&query);
    let list = list_rect(ui.body, rules::listing::LOOT_HEADER);
    let (scroll, start, end) = apply_scroll(rows.len(), list[3], scroll);
    hook::revise(|s| s.scroll = scroll);
    for (i, id) in rows.iter().enumerate().take(end).skip(start) {
        let r = row_box(list, i, scroll);
        let stack = ItemStack::new(id, 1);
        game.native_gui_draw_item(batch, &stack, [r[0] + 2.0, r[1] + 2.0]);
        ink(game, batch, &label_of(id), [r[0] + 22.0, r[1] + 5.0], TEXT);
        let keep = !toss.iter().any(|t| rules::normalize_id(t) == *id);
        let kr = [r[0] + r[2] - 108.0, r[1] + 2.0, 50.0, 16.0];
        let cr = [r[0] + r[2] - 54.0, r[1] + 2.0, 50.0, 16.0];
        chip(game, batch, kr, if keep { "Keep" } else { "Toss" }, true, hit(p, kr), if keep { KEEP } else { TOSS });
        let ringing = chime.iter().any(|c| rules::normalize_id(c) == *id);
        chip(game, batch, cr, if ringing { "Chime" } else { "Quiet" }, ringing, hit(p, cr), if ringing { ACCENT } else { CHIP });
    }
    paint_bar(game, batch, list, scroll, rows.len(), bar_hover(p, list, scroll, rows.len()));
}

fn charm_label(floor: i32) -> String {
    if floor == CHARM_TOSS {
        "Toss".into()
    } else if floor == CHARM_ANY {
        "Any".into()
    } else {
        format!("≥{floor}")
    }
}

fn paint_charms(game: &Game<'_>, batch: &mut HudBatch, ui: &Frame, p: (f32, f32)) {
    ink(game, batch, "Used when a book, rod, or bow row is Keep.", [ui.body[0], ui.body[1] + 2.0], DIM);
    let (scroll, floors) = hook::revise(|s| (s.scroll, s.prefs.charm_floor.clone()));
    let list = list_rect(ui.body, rules::listing::CHARM_HEADER);
    let (scroll, start, end) = apply_scroll(CHARMS.len(), list[3], scroll);
    hook::revise(|s| s.scroll = scroll);
    for (i, (name, max, _)) in CHARMS.iter().enumerate().take(end).skip(start) {
        let r = row_box(list, i, scroll);
        ink(game, batch, name, [r[0] + 4.0, r[1] + 5.0], TEXT);
        let key = rules::catalog::charm_slug(name);
        let floor = floors.get(&key).copied().unwrap_or(CHARM_ANY);
        let chip_r = [r[0] + r[2] - 70.0, r[1] + 2.0, 66.0, 16.0];
        let on_toss = floor == CHARM_TOSS;
        chip(
            game,
            batch,
            chip_r,
            &charm_label(floor),
            true,
            hit(p, chip_r),
            if on_toss { TOSS } else if floor == CHARM_ANY { KEEP } else { TAB_ON },
        );
        let _ = max;
    }
    paint_bar(game, batch, list, scroll, CHARMS.len(), bar_hover(p, list, scroll, CHARMS.len()));
}

fn paint_habits(game: &Game<'_>, batch: &mut HudBatch, ui: &Frame, p: (f32, f32)) {
    let flags: Vec<bool> = hook::revise(|s| hook::HABITS.iter().map(|(k, _)| hook::habit_on(s, k)).collect());
    for (i, ((_, label), on)) in hook::HABITS.iter().zip(flags).enumerate() {
        let r = [ui.body[0], ui.body[1] + 4.0 + i as f32 * 24.0, ui.body[2], 22.0];
        ink(game, batch, label, [r[0] + 4.0, r[1] + 5.0], TEXT);
        let chip_r = [r[0] + r[2] - 52.0, r[1] + 3.0, 48.0, 16.0];
        chip(game, batch, chip_r, if on { "On" } else { "Off" }, on, hit(p, chip_r), if on { KEEP } else { CHIP });
    }
}

fn paint_ledger(game: &Game<'_>, batch: &mut HudBatch, ui: &Frame, p: (f32, f32)) {
    let lines = hook::revise(|s| {
        vec![
            format!("Fish  {}", s.tally.fish),
            format!("Treasure  {}", s.tally.treasure),
            format!("Junk  {}", s.tally.junk),
            format!("Hooks  {}   Casts  {}", s.tally.hooks, s.tally.casts),
            format!("Hook rate  {}   Treasure rate  {}", s.tally.hook_rate(), s.tally.treasure_rate()),
            format!("Time running  {}s   XP guess  {}", s.tally.ticks / 20, s.tally.xp_guess),
        ]
    });
    for (i, line) in lines.iter().enumerate() {
        ink(game, batch, line, [ui.body[0] + 6.0, ui.body[1] + 8.0 + i as f32 * 18.0], TEXT);
    }
    let clear = [ui.body[0] + ui.body[2] - 110.0, ui.body[1] + ui.body[3] - 22.0, 110.0, 18.0];
    chip(game, batch, clear, "Clear ledger", hit(p, clear), hit(p, clear), TOSS);
}

fn save(game: &Game<'_>) {
    hook::revise(|s| hook::write_prefs_and_native(game, s));
}

pub fn click(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32) -> bool {
    if !ours(game, screen) || button != 0 {
        return false;
    }
    let ui = frame(screen);
    let p = (screen.mouse[0], screen.mouse[1]);
    if hit(p, ui.done) {
        hook::revise(|s| s.want_close = true);
        return true;
    }
    for (i, r) in ui.tabs.iter().enumerate() {
        if hit(p, *r) {
            hook::revise(|s| {
                s.tab = i as u8;
                s.scroll = 0.0;
                s.bar_grab = None;
                s.query_focus = false;
            });
            return true;
        }
    }
    let tab = hook::revise(|s| s.tab);
    match tab {
        0 => click_loot(game, &ui, p),
        1 => click_charms(game, &ui, p),
        2 => click_habits(game, &ui, p),
        _ => click_ledger(game, &ui, p),
    }
}

fn click_loot(game: &Game<'_>, ui: &Frame, p: (f32, f32)) -> bool {
    if hit(p, ui.search) {
        hook::revise(|s| s.query_focus = true);
        return true;
    }
    hook::revise(|s| s.query_focus = false);
    let (query, scroll, _toss) = hook::revise(|s| (s.query.clone(), s.scroll, s.prefs.toss.clone()));
    let rows = loot_rows(&query);
    let list = list_rect(ui.body, rules::listing::LOOT_HEADER);
    let (scroll, start, end) = apply_scroll(rows.len(), list[3], scroll);
    if click_bar(p, list, rows.len(), scroll) {
        return true;
    }
    for (i, id) in rows.iter().enumerate().take(end).skip(start) {
        let r = row_box(list, i, scroll);
        let kr = [r[0] + r[2] - 108.0, r[1] + 2.0, 50.0, 16.0];
        let cr = [r[0] + r[2] - 54.0, r[1] + 2.0, 50.0, 16.0];
        if hit(p, kr) {
            hook::revise(|s| {
                let id = (*id).to_string();
                if s.prefs.toss.contains(&id) {
                    s.prefs.toss.remove(&id);
                } else {
                    s.prefs.toss.insert(id);
                }
            });
            save(game);
            return true;
        }
        if hit(p, cr) {
            hook::revise(|s| {
                let id = (*id).to_string();
                if s.prefs.chime.contains(&id) {
                    s.prefs.chime.remove(&id);
                } else {
                    s.prefs.chime.insert(id);
                }
            });
            save(game);
            return true;
        }
    }
    true
}

fn click_charms(game: &Game<'_>, ui: &Frame, p: (f32, f32)) -> bool {
    let scroll = hook::revise(|s| s.scroll);
    let list = list_rect(ui.body, rules::listing::CHARM_HEADER);
    let (scroll, start, end) = apply_scroll(CHARMS.len(), list[3], scroll);
    if click_bar(p, list, CHARMS.len(), scroll) {
        return true;
    }
    for (i, (name, max, _)) in CHARMS.iter().enumerate().take(end).skip(start) {
        let r = row_box(list, i, scroll);
        let chip_r = [r[0] + r[2] - 70.0, r[1] + 2.0, 66.0, 16.0];
        if hit(p, chip_r) {
            let key = rules::catalog::charm_slug(name);
            hook::revise(|s| {
                let cur = s.prefs.charm_floor.get(&key).copied().unwrap_or(CHARM_ANY);
                s.prefs.charm_floor.insert(key, cycle_charm_floor(cur, *max));
            });
            save(game);
            return true;
        }
    }
    true
}

fn click_habits(game: &Game<'_>, ui: &Frame, p: (f32, f32)) -> bool {
    for (i, (key, _)) in hook::HABITS.iter().enumerate() {
        let r = [ui.body[0], ui.body[1] + 4.0 + i as f32 * 24.0, ui.body[2], 22.0];
        let chip_r = [r[0] + r[2] - 52.0, r[1] + 3.0, 48.0, 16.0];
        if hit(p, chip_r) || hit(p, r) {
            hook::revise(|s| {
                hook::flip_habit(s, key);
            });
            save(game);
            return true;
        }
    }
    true
}

fn click_ledger(game: &Game<'_>, ui: &Frame, p: (f32, f32)) -> bool {
    let clear = [ui.body[0] + ui.body[2] - 110.0, ui.body[1] + ui.body[3] - 22.0, 110.0, 18.0];
    if hit(p, clear) {
        hook::revise(|s| s.tally = rules::SessionTally::default());
        save(game);
        return true;
    }
    true
}

pub fn wheel(game: &Game<'_>, screen: &NativeScreenView<'_>, scroll: [f64; 2]) -> bool {
    if !ours(game, screen) {
        return false;
    }
    let dy = if scroll[1].abs() > scroll[0].abs() { scroll[1] } else { scroll[0] };
    if dy == 0.0 {
        return true;
    }
    let ui = frame(screen);
    hook::revise(|s| {
        let (total, list) = list_totals(&ui, s.tab, &s.query);
        s.scroll = rules::listing::clamp_scroll(s.scroll - dy as f32 * 16.0, total, list[3]);
    });
    true
}

pub fn drag(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32) -> bool {
    if !ours(game, screen) || button != 0 {
        return false;
    }
    let grab = hook::revise(|s| s.bar_grab);
    let Some(off) = grab else {
        return false;
    };
    let ui = frame(screen);
    let p = (screen.mouse[0], screen.mouse[1]);
    hook::revise(|s| {
        let (total, list) = list_totals(&ui, s.tab, &s.query);
        let track = bar_track(list);
        s.scroll = rules::listing::scroll_from_drag(p.1 - track[1], off, total, list[3], track[3]);
    });
    true
}

pub fn release(game: &Game<'_>, screen: &NativeScreenView<'_>, button: i32) -> bool {
    if !ours(game, screen) {
        return false;
    }
    hook::revise(|s| {
        s.bar_grab = None;
    });
    let _ = (game, button);
    true
}

pub fn key(game: &Game<'_>, screen: &NativeScreenView<'_>, key: &str) -> bool {
    if !ours(game, screen) {
        return false;
    }
    if key == "key.keyboard.escape" {
        hook::revise(|s| s.want_close = true);
        return true;
    }
    let focus = hook::revise(|s| s.query_focus && s.tab == 0);
    if !focus {
        return true;
    }
    if key == "key.keyboard.backspace" {
        hook::revise(|s| {
            s.query.pop();
        });
        return true;
    }
    let _ = game;
    true
}

pub fn typed(game: &Game<'_>, screen: &NativeScreenView<'_>, c: char) -> bool {
    if !ours(game, screen) {
        return false;
    }
    let focus = hook::revise(|s| s.query_focus && s.tab == 0);
    if !focus {
        return true;
    }
    if !c.is_control() && c != '§' {
        hook::revise(|s| {
            if s.query.len() < 24 {
                s.query.push(c);
            }
        });
    }
    let _ = game;
    true
}

pub fn close_if_asked(game: &mut Game<'_>) {
    let close = hook::revise(|s| {
        let c = s.want_close;
        s.want_close = false;
        c
    });
    if close && game.native_mod_screen() == Some(hook::MOD_ID) {
        game.native_mod_screen_close(hook::MOD_ID);
    }
}
