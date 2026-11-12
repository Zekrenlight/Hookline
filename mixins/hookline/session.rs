// Horizon-facing Hookline session: packets, inventory, meals, and ticks.
// Rules live in `hookline_logic`; this file only talks to the game.

use crate::game::hookline_logic as rules;
use rules::{
    ballots_for, bite_from_splash, default_toss_set, dump_prefs, hunger_needs_food, is_fish,
    is_rod, is_treasure, load_prefs, may_press_use, pick_food_origin, recast_pause, should_stow,
    should_toss, sort_catch, splash_sound, step_meal, ticks_until_use, BiteCue, CatchKind,
    FoodOrigin, MealPhase, MealStep, Prefs, SessionTally, FORMAT, LULL_TICKS, MEAL_HOLD_TICKS,
};
use rules::catalog::{is_risky_food, label_of, normalize_id, rod_is_worn};
use rules::detect::{range_sq, settle_after_cast, splash_is_ours};
use rules::persist::{apply_native_overlay, FILE_NAME};
use rules::policy::{may_begin_stow, stow_may_click};
use rules::timing::{
    meal_after_swap_pause, meal_between_pause, meal_pre_pause, meal_recast_pause,
    meal_swap_back_pause, mix64, HURT_LEAVE, NO_FOOD_EVERY, STOW_COOLDOWN, WANDER_PERIOD,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const MOD_ID: &str = "hookline";
pub const TITLE: &str = "Hookline";
pub const VERSION: &str = "1.0.0";
pub const BLURB: &str = "Reels your own bobber, recasts, and lets you keep, toss, deposit, or eat on your terms.";
pub const AUTHOR: &str = "Zekrenlight";

pub const BINDINGS: &[[&str; 3]] = &[
    ["toggle", "Turn Hookline on or off", rules::persist::TOGGLE_KEY],
    ["panel", "Open the Hookline panel", rules::persist::PANEL_KEY],
];

pub const HABITS: &[(&str, &str)] = &[
    ("pace", "Pace variance (softer reel and recast timing)"),
    ("wander", "Look around and hop once a minute"),
    ("meals", "Eat from off-hand or hotbar until full"),
    ("worn_rod", "Switch or stop on a worn rod"),
    ("bail_hurt", "Leave the server below three hearts"),
    ("rest_after_lull", "Rest after five quiet minutes"),
    ("overlay", "Corner overlay"),
];

const ON_MSG: &str = "Hookline is on";
const OFF_MSG: &str = "Hookline is off";
const LULL_MSG: &str = "Hookline paused after a quiet stretch";
const NO_FOOD_MSG: &str = "Hookline: nothing to eat";
const SEED: u64 = 0x484F_4F4B_4C49_4E31;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StowStep {
    Start,
    Hunt,
    Aim,
    Transfer,
    Recover,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub prefs: Prefs,
    pub ready: bool,
    pub tally: SessionTally,
    pub note: String,
    pub flash: Option<(String, i32)>,
    pub seed: u64,
    pub reel_in: i32,
    pub recast_in: i32,
    pub settle: i32,
    pub watch_loot: i32,
    pub lull: i32,
    pub ages_release: i32,
    pub ages_slot: i32,
    pub ages_throw: i32,
    pub meal_phase: MealPhase,
    pub meal_timer: i32,
    pub meal_slot: i32,
    pub meal_hand: i32,
    pub meal_holding: bool,
    pub no_food: i32,
    pub wander: i32,
    pub prior: HashMap<String, u32>,
    pub counts_dirty: bool,
    pub stowing: bool,
    pub stow_step: StowStep,
    pub stow_wait: i32,
    pub stow_index: usize,
    pub stow_cool: i32,
    pub we_opened: bool,
    pub we_clicked: bool,
    pub saved_yaw: f32,
    pub saved_pitch: f32,
    pub chest: Option<[i32; 3]>,
    pub tab: u8,
    pub scroll: f32,
    pub bar_grab: Option<f32>,
    pub query: String,
    pub query_focus: bool,
    pub want_close: bool,
    pub key_down: HashMap<String, bool>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            prefs: Prefs::default(),
            ready: false,
            tally: SessionTally::default(),
            note: String::new(),
            flash: None,
            seed: SEED,
            reel_in: -1,
            recast_in: -1,
            settle: 0,
            watch_loot: 0,
            lull: 0,
            ages_release: 100,
            ages_slot: 100,
            ages_throw: 100,
            meal_phase: MealPhase::Rest,
            meal_timer: 0,
            meal_slot: -1,
            meal_hand: 0,
            meal_holding: false,
            no_food: 0,
            wander: 0,
            prior: HashMap::new(),
            counts_dirty: false,
            stowing: false,
            stow_step: StowStep::Start,
            stow_wait: 0,
            stow_index: 0,
            stow_cool: 0,
            we_opened: false,
            we_clicked: false,
            saved_yaw: 0.0,
            saved_pitch: 0.0,
            chest: None,
            tab: 0,
            scroll: 0.0,
            bar_grab: None,
            query: String::new(),
            query_focus: false,
            want_close: false,
            key_down: HashMap::new(),
        }
    }
}

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::default());
}

pub fn revise<R>(f: impl FnOnce(&mut Session) -> R) -> R {
    SESSION.with(|c| f(&mut c.borrow_mut()))
}

fn pulse(s: &mut Session, text: impl Into<String>) {
    s.flash = Some((text.into(), 40));
}

pub fn tick_flash(s: &mut Session) {
    if let Some((_, t)) = s.flash.as_mut() {
        *t -= 1;
        if *t <= 0 {
            s.flash = None;
        }
    }
}

fn say(game: &mut crate::game::Game<'_>, text: &str) {
    game.native_client_action_bar(horizon_client::text::Text::literal(text), false);
}

pub fn habit_on(s: &Session, key: &str) -> bool {
    match key {
        "pace" => s.prefs.pace,
        "wander" => s.prefs.wander,
        "meals" => s.prefs.meals,
        "worn_rod" => s.prefs.worn_rod,
        "bail_hurt" => s.prefs.bail_hurt,
        "rest_after_lull" => s.prefs.rest_after_lull,
        "overlay" => s.prefs.overlay,
        _ => false,
    }
}

pub fn set_habit(s: &mut Session, key: &str, on: bool) {
    match key {
        "pace" => s.prefs.pace = on,
        "wander" => s.prefs.wander = on,
        "meals" => s.prefs.meals = on,
        "worn_rod" => s.prefs.worn_rod = on,
        "bail_hurt" => s.prefs.bail_hurt = on,
        "rest_after_lull" => s.prefs.rest_after_lull = on,
        "overlay" => s.prefs.overlay = on,
        _ => {}
    }
}

pub fn flip_habit(s: &mut Session, key: &str) -> bool {
    let next = !habit_on(s, key);
    set_habit(s, key, next);
    next
}

fn yes_no(on: bool) -> &'static str {
    if on { "yes" } else { "no" }
}

pub fn write_prefs(s: &mut Session, dir: &Path) {
    s.prefs.format = s.prefs.format.max(FORMAT);
    let _ = fs::create_dir_all(dir);
    let _ = fs::write(dir.join(FILE_NAME), dump_prefs(&s.prefs));
}

pub fn write_prefs_and_native(game: &crate::game::Game<'_>, s: &mut Session) {
    let dir = game.native_data_dir(MOD_ID);
    write_prefs(s, &dir);
    for (key, _) in HABITS {
        let _ = game.native_set_setting(MOD_ID, key, yes_no(habit_on(s, key)));
    }
}

pub fn ensure_ready(game: &crate::game::Game<'_>) {
    let needed = revise(|s| !s.ready);
    if !needed {
        return;
    }
    let dir = game.native_data_dir(MOD_ID);
    let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap_or_default();
    let (mut prefs, file_keys) = load_prefs(&text);
    let mut native = HashMap::new();
    for (key, _) in HABITS {
        if let Some(v) = game.native_setting(MOD_ID, key) {
            native.insert((*key).to_string(), v);
        }
    }
    apply_native_overlay(&mut prefs, &native, &file_keys);
    prefs.format = prefs.format.max(FORMAT);
    revise(|s| {
        s.prefs = prefs;
        s.ready = true;
        let dir = game.native_data_dir(MOD_ID);
        write_prefs(s, &dir);
    });
}

pub fn rising(name: &str, pressed: bool) -> bool {
    revise(|s| {
        let was = s.key_down.insert(name.to_string(), pressed).unwrap_or(false);
        pressed && !was
    })
}

pub fn mark_activity(s: &mut Session) {
    s.lull = 0;
}

fn age_actions(s: &mut Session) {
    s.ages_release = s.ages_release.saturating_add(1);
    s.ages_slot = s.ages_slot.saturating_add(1);
    s.ages_throw = s.ages_throw.saturating_add(1);
}

fn can_use(s: &mut Session) -> bool {
    may_press_use(s.ages_release, s.ages_slot, s.ages_throw)
}

fn gap(s: &Session) -> i32 {
    ticks_until_use(s.ages_release, s.ages_slot, s.ages_throw)
}

fn halt(s: &mut Session) {
    s.prefs.running = false;
    s.reel_in = -1;
    s.recast_in = -1;
    s.settle = 0;
    s.stowing = false;
    s.we_opened = false;
    s.we_clicked = false;
    s.meal_holding = false;
    s.meal_phase = MealPhase::Rest;
    s.meal_timer = 0;
    s.lull = 0;
}

pub fn halt_in_world(game: &mut crate::game::Game<'_>) {
    let (close, let_go) = revise(|s| {
        let close = s.stowing;
        let let_go = s.meal_holding;
        if close {
            game.yaw = s.saved_yaw;
            game.pitch = s.saved_pitch;
        }
        halt(s);
        (close && game.container.is_some(), let_go)
    });
    if let_go {
        game.right = false;
        revise(|s| s.ages_release = 0);
    }
    if close {
        game.close_container();
    }
}

fn toggle_running(s: &mut Session) -> &'static str {
    s.prefs.running = !s.prefs.running;
    if s.prefs.running {
        s.counts_dirty = true;
        s.lull = 0;
        s.note = ON_MSG.into();
        ON_MSG
    } else {
        s.note = OFF_MSG.into();
        OFF_MSG
    }
}

pub fn help_text() -> Vec<String> {
    vec![
        "Hookline commands: /hookline, /hookline toggle, /hookline pace, /hookline ledger, /hookline clear-ledger, /hookline toss, /hookline chime, /hookline panel.".into(),
        "/hookline toss and /hookline chime add or remove the item in your main hand. /hookline panel opens the tabbed card.".into(),
    ]
}

fn stack_charms(stack: &horizon_client::items::ItemStack) -> Vec<(String, i32)> {
    stack
        .crafting_enchantments()
        .iter()
        .map(|(id, lvl)| (id.to_string(), *lvl as i32))
        .collect()
}

fn max_damage(game: &crate::game::Game<'_>, item: &str) -> u32 {
    game.res.items().get(item).map(|i| i.max_damage).unwrap_or(0)
}

fn is_food_item(game: &crate::game::Game<'_>, item: &str) -> bool {
    game.res.items().get(item).is_some_and(|info| info.food.is_some())
}

fn held_rod(game: &crate::game::Game<'_>) -> Option<horizon_client::items::ItemStack> {
    let stack = game.inventory.get(game.selected).and_then(|s| s.clone())?;
    is_rod(&stack.item).then_some(stack)
}

fn find_rod(game: &crate::game::Game<'_>, require_health: bool) -> Option<usize> {
    for slot in 0..9 {
        let Some(stack) = game.inventory.get(slot).and_then(|s| s.as_ref()) else {
            continue;
        };
        if !is_rod(&stack.item) {
            continue;
        }
        if require_health {
            let max = max_damage(game, &stack.item);
            if rod_is_worn(stack.damage, max) {
                continue;
            }
        }
        return Some(slot);
    }
    None
}

pub fn change_slot(game: &mut crate::game::Game<'_>, slot: usize) {
    if slot > 8 {
        return;
    }
    if game.selected != slot {
        revise(|s| s.ages_slot = 0);
    }
    game.selected = slot;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    let packet = S::SetCarriedItem(horizon_protocol::packets::play::SetCarriedItem { slot: slot as i16 });
    let _ = game.native_send_packet(packet);
}

fn press_use_rod(game: &mut crate::game::Game<'_>, stack: &horizon_client::items::ItemStack) -> bool {
    if !is_rod(&stack.item) {
        return false;
    }
    if !revise(can_use) {
        return false;
    }
    if game.remote.is_some() {
        use horizon_client::mod_api::ServerboundPlayPacket as S;
        let packet = S::UseItem(horizon_protocol::packets::play::UseItem {
            hand: 0,
            sequence: 0,
            yaw: game.yaw,
            pitch: game.pitch,
        });
        game.native_send_packet(packet).is_ok()
    } else {
        game.use_fishing_rod(stack)
    }
}

fn press_use_hand(game: &mut crate::game::Game<'_>, hand: i32) -> bool {
    if !revise(can_use) {
        return false;
    }
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    let packet = S::UseItem(horizon_protocol::packets::play::UseItem {
        hand,
        sequence: 0,
        yaw: game.yaw,
        pitch: game.pitch,
    });
    let sent = if game.remote.is_some() {
        game.native_send_packet(packet).is_ok()
    } else {
        let _ = game.native_send_packet(packet);
        true
    };
    if game.using.is_none() {
        if hand == 0 {
            let _ = game.start_using_item();
        } else {
            game.using = Some((40, 0));
        }
    }
    sent
}

fn pick_usable_rod(game: &mut crate::game::Game<'_>) -> bool {
    let stop_worn = revise(|s| s.prefs.worn_rod);
    if let Some(stack) = game.inventory.get(game.selected).and_then(|s| s.as_ref()) {
        if is_rod(&stack.item) {
            if stop_worn {
                let max = max_damage(game, &stack.item);
                if !rod_is_worn(stack.damage, max) {
                    return true;
                }
            } else {
                return true;
            }
        }
    }
    if let Some(slot) = find_rod(game, stop_worn) {
        change_slot(game, slot);
        revise(|s| {
            pulse(s, "Switched to a fishing rod");
            s.recast_in = 10;
        });
        game.native_chat("Hookline: switched to a fishing rod.");
        false
    } else {
        revise(|s| {
            halt(s);
            s.recast_in = -1;
            pulse(s, "Hookline stopped: no usable rod in the hotbar");
            let dir = game.native_data_dir(MOD_ID);
            write_prefs(s, &dir);
        });
        game.native_chat(
            "Hookline stopped: no usable fishing rod in the hotbar (rods only in the bag are not swapped automatically).",
        );
        false
    }
}

fn after_hook(s: &mut Session) {
    mark_activity(s);
    s.tally.hooks += 1;
    s.tally.casts += 1;
    s.watch_loot = 40;
    s.reel_in = -1;
    s.recast_in = recast_pause(&mut s.seed, s.prefs.pace);
    s.note = format!("Reeled. Recast in {} ticks", s.recast_in);
}

fn after_cast(s: &mut Session) {
    mark_activity(s);
    s.tally.casts += 1;
    s.recast_in = -1;
    s.settle = settle_after_cast();
    s.note = "Line is out.".into();
}

pub fn try_reel_now(game: &mut crate::game::Game<'_>) {
    let Some(stack) = game.inventory.get(game.selected).and_then(|s| s.clone()) else {
        return;
    };
    if !is_rod(&stack.item) || game.hook.is_none() {
        return;
    }
    if !revise(can_use) {
        revise(|s| s.reel_in = gap(s).max(1));
        return;
    }
    if press_use_rod(game, &stack) {
        revise(after_hook);
    }
}

fn tick_reel(game: &mut crate::game::Game<'_>) -> bool {
    let due = revise(|s| {
        if s.reel_in > 0 {
            s.reel_in -= 1;
            s.reel_in == 0
        } else {
            false
        }
    });
    if !due {
        return false;
    }
    if game.hook.is_none() {
        revise(|s| s.reel_in = -1);
        return false;
    }
    if !revise(can_use) {
        revise(|s| s.reel_in = gap(s).max(1));
        return true;
    }
    if let Some(rod) = held_rod(game) {
        if press_use_rod(game, &rod) {
            revise(after_hook);
        }
    }
    true
}

fn tick_recast(game: &mut crate::game::Game<'_>) {
    let due = revise(|s| {
        if s.recast_in > 0 {
            s.recast_in -= 1;
            s.recast_in == 0
        } else {
            false
        }
    });
    if !due {
        return;
    }
    if game.hook.is_some() {
        revise(|s| s.recast_in = 5);
        return;
    }
    if !pick_usable_rod(game) {
        return;
    }
    if !revise(can_use) {
        revise(|s| s.recast_in = gap(s).max(1));
        return;
    }
    if let Some(rod) = held_rod(game) {
        if press_use_rod(game, &rod) {
            revise(after_cast);
        }
    }
}

fn use_key_down(game: &mut crate::game::Game<'_>) -> bool {
    game.native_mapping("key.use").map(|(down, _)| down).unwrap_or(false)
}

fn gather_food(game: &crate::game::Game<'_>) -> Option<FoodOrigin> {
    let offhand = game.offhand.as_ref().and_then(|s| {
        if s.count > 0 && is_food_item(game, &s.item) {
            Some(s.item.to_string())
        } else {
            None
        }
    });
    let mut hotbar: [Option<String>; 9] = Default::default();
    for slot in 0..9 {
        if let Some(s) = game.inventory.get(slot).and_then(|s| s.as_ref()) {
            if s.count > 0 && &*s.item != "minecraft:air" && is_food_item(game, &s.item) {
                hotbar[slot] = Some(s.item.to_string());
            }
        }
    }
    pick_food_origin(offhand.as_deref(), &hotbar, game.selected)
}

fn remember_food(s: &mut Session, src: &FoodOrigin) {
    s.meal_slot = src.chew_slot();
    s.meal_hand = src.hand();
}

fn prepare_food(game: &mut crate::game::Game<'_>, src: &FoodOrigin) {
    match src {
        FoodOrigin::Offhand => {}
        FoodOrigin::Hotbar { slot } => change_slot(game, *slot),
    }
    revise(|s| remember_food(s, src));
}

fn restore_rod(game: &mut crate::game::Game<'_>) {
    if let Some(slot) = find_rod(game, false) {
        change_slot(game, slot);
    }
}

fn cancel_meal(game: &mut crate::game::Game<'_>) {
    let holding = revise(|s| s.meal_holding);
    if holding {
        game.right = false;
        revise(|s| s.ages_release = 0);
    }
    restore_rod(game);
    revise(|s| {
        clear_meal(s);
        s.recast_in = recast_pause(&mut s.seed, s.prefs.pace);
        s.note = "Meal cancelled.".into();
    });
}

fn clear_meal(s: &mut Session) {
    s.meal_timer = 0;
    s.meal_slot = -1;
    s.meal_hand = 0;
    s.meal_holding = false;
    s.meal_phase = MealPhase::Rest;
}

fn tick_meal(game: &mut crate::game::Game<'_>) -> bool {
    let food_level = game.player.as_ref().map(|p| p.health.food.food).unwrap_or(20);
    let health = game.player.as_ref().map(|p| p.health.health()).unwrap_or(20.0);
    let max_health = 20.0_f32.max(health);
    let hungry = hunger_needs_food(food_level, health, max_health);
    let player_use = use_key_down(game);
    let source = gather_food(game);
    let has_food = source.is_some();
    let need_swap = source.as_ref().is_some_and(|s| s.needs_swap(game.selected));
    let hook_out = game.hook.is_some();
    let (phase, timer, we_hold, still, meals, can, pre, after, between, back, recast) = revise(|s| {
        if s.no_food > 0 {
            s.no_food -= 1;
        }
        let still = if s.meal_hand == 1 {
            true
        } else if s.meal_slot >= 0 {
            game.selected == s.meal_slot as usize
        } else {
            true
        };
        let pace = s.prefs.pace;
        (
            s.meal_phase,
            s.meal_timer,
            s.meal_holding,
            still,
            s.prefs.meals,
            can_use(s),
            meal_pre_pause(&mut s.seed, pace),
            meal_after_swap_pause(&mut s.seed, pace),
            meal_between_pause(&mut s.seed, pace),
            meal_swap_back_pause(&mut s.seed, pace),
            meal_recast_pause(&mut s.seed, pace),
        )
    });
    let adv = step_meal(
        phase, timer, meals, hungry, has_food, hook_out, player_use && !we_hold, still, can,
        need_swap, pre, after, between, back, recast,
    );
    match adv.step {
        MealStep::Idle => {
            if phase == MealPhase::Rest && adv.phase == MealPhase::Rest && meals && hungry && !has_food {
                let now = revise(|s| {
                    if s.no_food > 0 {
                        false
                    } else {
                        s.no_food = NO_FOOD_EVERY;
                        true
                    }
                });
                if now {
                    say(game, NO_FOOD_MSG);
                }
            }
            if adv.phase == MealPhase::Chewing || we_hold {
                game.right = true;
            }
            revise(|s| {
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
            });
            adv.phase.busy()
        }
        MealStep::Reel => {
            if let Some(rod) = held_rod(game) {
                if !press_use_rod(game, &rod) {
                    return true;
                }
            }
            revise(|s| {
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
                s.reel_in = -1;
                s.recast_in = -1;
                s.note = "Getting ready to eat...".into();
            });
            true
        }
        MealStep::SwapFood => {
            let Some(src) = source else {
                cancel_meal(game);
                return false;
            };
            prepare_food(game, &src);
            revise(|s| {
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
                s.reel_in = -1;
                s.note = "Getting ready to eat...".into();
            });
            true
        }
        MealStep::StartChew => {
            let Some(src) = source else {
                cancel_meal(game);
                return false;
            };
            if revise(|s| s.meal_slot < 0 && s.meal_hand == 0) {
                revise(|s| remember_food(s, &src));
            }
            game.right = true;
            if !press_use_hand(game, src.hand()) {
                game.right = false;
                revise(|s| {
                    s.meal_holding = false;
                    s.meal_phase = MealPhase::BeforeUse;
                    s.meal_timer = 1;
                });
                return true;
            }
            revise(|s| {
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
                s.meal_holding = true;
                s.reel_in = -1;
                s.note = "Eating...".into();
                remember_food(s, &src);
            });
            true
        }
        MealStep::KeepChew => {
            game.right = true;
            revise(|s| {
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
                s.meal_holding = true;
            });
            true
        }
        MealStep::LetGo => {
            game.right = false;
            revise(|s| {
                s.ages_release = 0;
                s.meal_holding = false;
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
                s.note = if adv.phase == MealPhase::BetweenBites {
                    "Pausing between bites...".into()
                } else {
                    "Finished eating...".into()
                };
            });
            true
        }
        MealStep::SwapRod => {
            restore_rod(game);
            revise(|s| {
                s.meal_phase = adv.phase;
                s.meal_timer = adv.timer;
                s.meal_slot = -1;
                s.note = format!("Finished eating; recasting in {} ticks", adv.timer);
            });
            true
        }
        MealStep::CastAgain => {
            revise(clear_meal);
            if !pick_usable_rod(game) {
                return true;
            }
            if let Some(rod) = held_rod(game) {
                if press_use_rod(game, &rod) {
                    revise(after_cast);
                } else {
                    revise(|s| s.recast_in = gap(s).max(2));
                }
            }
            true
        }
        MealStep::Cancel => {
            cancel_meal(game);
            false
        }
    }
}

fn toss_slot(game: &mut crate::game::Game<'_>, slot: usize) {
    if slot >= 36 {
        return;
    }
    revise(|s| s.ages_throw = 0);
    if game.remote.is_some() {
        let _ = game.remote_menu_ops(&[(Some(slot), 1, horizon_client::menu::Input::Throw)]);
        return;
    }
    if game.container.is_some() || game.screen == crate::game::Screen::Inventory {
        let menu = menu_slot(game, slot);
        let _ = game.menu_click(Some(menu), 1, horizon_client::menu::Input::Throw);
        return;
    }
    if slot < 9 {
        let prev = game.selected;
        if prev == slot
            && game
                .inventory
                .get(slot)
                .and_then(|s| s.as_ref())
                .is_some_and(|s| is_rod(&s.item))
        {
            return;
        }
        change_slot(game, slot);
        if game.remote.is_some() {
            game.remote_drop(true);
        } else {
            game.drop_held(true);
        }
        if prev < 9 {
            change_slot(game, prev);
        }
        return;
    }
    let _ = game.menu_click(Some(slot), 1, horizon_client::menu::Input::Throw);
}

fn notice_bags(game: &mut crate::game::Game<'_>) {
    let mut seen: Vec<(usize, String, u32, Vec<(String, i32)>)> = Vec::new();
    for (slot, stack) in game.inventory.iter().take(36).enumerate() {
        let Some(stack) = stack else { continue };
        if stack.count == 0 || &*stack.item == "minecraft:air" {
            continue;
        }
        seen.push((slot, stack.item.to_string(), stack.count, stack_charms(stack)));
    }
    let mut chimes = Vec::new();
    let mut drops = Vec::new();
    revise(|s| {
        let mut now = HashMap::new();
        for (slot, id, count, charms) in &seen {
            let key = format!("{slot}:{id}");
            let prev = if s.counts_dirty { *count } else { *s.prior.get(&key).unwrap_or(&0) };
            if *count > prev && s.watch_loot > 0 {
                match sort_catch(id) {
                    CatchKind::Treasure => s.tally.treasure += 1,
                    CatchKind::Fish => s.tally.fish += 1,
                    CatchKind::Junk => s.tally.junk += 1,
                }
                s.tally.xp_guess += (mix64(&mut s.seed) % 5) as u32 + 1;
            }
            if *count > prev && s.prefs.chime.iter().any(|c| normalize_id(c) == normalize_id(id)) {
                chimes.push(id.clone());
            }
            let in_use = is_rod(id) && *slot == game.selected;
            if s.prefs.running
                && should_toss(id, &s.prefs.toss, charms, &s.prefs.charm_floor, in_use)
            {
                drops.push((*slot, id.clone()));
            }
            now.insert(key, *count);
        }
        s.counts_dirty = false;
        s.prior = now;
        if s.watch_loot > 0 {
            s.watch_loot -= 1;
        }
    });
    let pos = player_xyz(game);
    for item in chimes {
        let _ = game.native_play_local_sound(
            "minecraft:entity.experience_orb.pickup",
            pos,
            1.0,
            1.0,
            0,
        );
        let label = label_of(&item);
        let label = if label.is_empty() { item.as_str() } else { label.as_str() };
        let msg = format!("Caught: {label}");
        revise(|s| pulse(s, msg.clone()));
        say(game, &msg);
    }
    if !drops.is_empty() {
        let names: Vec<_> = drops.iter().map(|(_, n)| label_of(n)).collect();
        revise(|s| pulse(s, format!("Tossing: {}", names.join(", "))));
        for (slot, _) in drops {
            toss_slot(game, slot);
        }
    }
}

fn free_slots(game: &crate::game::Game<'_>) -> usize {
    game.inventory
        .iter()
        .take(36)
        .filter(|s| match s {
            None => true,
            Some(st) => st.count == 0 || &*st.item == "minecraft:air",
        })
        .count()
}

fn menu_slot(game: &crate::game::Game<'_>, inv: usize) -> usize {
    let total = game.native_screen_slot_count().unwrap_or(36);
    total.saturating_sub(36) + inv
}

fn player_xyz(game: &crate::game::Game<'_>) -> [f64; 3] {
    game.player
        .as_ref()
        .map(|p| [p.position.x, p.position.y, p.position.z])
        .or_else(|| game.native_player().map(|p| [p.x, p.y, p.z]))
        .unwrap_or([0.0, 0.0, 0.0])
}

fn block_id(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    game.res.facts().get(state).name.to_string()
}

fn is_bin(name: &str) -> bool {
    matches!(name, "minecraft:chest" | "minecraft:trapped_chest" | "minecraft:barrel")
}

fn nearby_bin(game: &crate::game::Game<'_>) -> Option<[i32; 3]> {
    let p = game.player.as_ref()?;
    let bx = p.position.x.floor() as i32;
    let by = p.position.y.floor() as i32;
    let bz = p.position.z.floor() as i32;
    for dy in -2..=1 {
        for dx in -2..=2 {
            for dz in -2..=2 {
                let pos = [bx + dx, by + dy, bz + dz];
                if is_bin(&block_id(game, pos)) {
                    return Some(pos);
                }
            }
        }
    }
    None
}

fn face_block(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    let Some(p) = game.player.as_ref() else { return };
    let eyes = [p.position.x, p.position.y + 1.62, p.position.z];
    let target = [pos[0] as f64 + 0.5, pos[1] as f64 + 0.5, pos[2] as f64 + 0.5];
    let dx = target[0] - eyes[0];
    let dy = target[1] - eyes[1];
    let dz = target[2] - eyes[2];
    let dist = (dx * dx + dz * dz).sqrt().max(0.0001);
    let yaw = (-dx.atan2(dz) as f32).to_degrees();
    let pitch = (-dy.atan2(dist) as f32).to_degrees();
    game.yaw = yaw;
    game.pitch = pitch;
    if let Some(pl) = game.player.as_mut() {
        pl.yaw = yaw;
        pl.pitch = pitch;
    }
}

fn click_bin(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    face_block(game, pos);
    if game.remote.is_some() {
        use horizon_client::mod_api::ServerboundPlayPacket as S;
        let packet = S::UseItemOn(horizon_protocol::packets::play::UseItemOn {
            hand: 0,
            pos,
            face: 1,
            cursor: [0.5, 1.0, 0.5],
            inside: false,
            world_border: false,
            sequence: 0,
        });
        let _ = game.native_send_packet(packet);
    } else {
        let hit = crate::game::Hit {
            pos,
            face: [0, 1, 0],
            point: [pos[0] as f64 + 0.5, pos[1] as f64 + 1.0, pos[2] as f64 + 0.5],
        };
        let _ = game.use_hand(Some(&hit));
    }
}

fn quick_stow(game: &mut crate::game::Game<'_>, inv: usize) {
    if game.remote.is_some() {
        let _ = game.remote_menu_ops(&[(Some(inv), 0, horizon_client::menu::Input::QuickMove)]);
    } else {
        let slot = menu_slot(game, inv);
        let _ = game.inventory_quick_move(slot);
    }
}

fn finish_stow(s: &mut Session) {
    s.stowing = false;
    s.stow_step = StowStep::Start;
    s.stow_wait = 0;
    s.stow_index = 0;
    s.we_opened = false;
    s.we_clicked = false;
    s.stow_cool = STOW_COOLDOWN;
    s.chest = None;
    s.recast_in = 15;
    s.note = "Deposit finished; recasting.".into();
}

fn begin_stow(s: &mut Session, yaw: f32, pitch: f32) {
    if !may_begin_stow(s.stowing, s.stow_cool) {
        return;
    }
    s.stowing = true;
    s.stow_step = StowStep::Start;
    s.stow_wait = 10;
    s.stow_index = 0;
    s.we_opened = false;
    s.we_clicked = false;
    s.saved_yaw = yaw;
    s.saved_pitch = pitch;
    s.chest = None;
    s.reel_in = -1;
    s.recast_in = -1;
    s.note = "Bag is tight: looking for a chest.".into();
}

fn should_stow_slot(game: &crate::game::Game<'_>, slot: usize) -> bool {
    let Some(stack) = game.inventory.get(slot).and_then(|s| s.as_ref()) else {
        return false;
    };
    if stack.count == 0 || &*stack.item == "minecraft:air" {
        return false;
    }
    let charms = stack_charms(stack);
    revise(|s| should_stow(&stack.item, &s.prefs.toss, &charms, &s.prefs.charm_floor))
}

fn run_stow(game: &mut crate::game::Game<'_>) {
    let step = revise(|s| s.stow_step);
    match step {
        StowStep::Start => {
            revise(|s| {
                s.stow_step = StowStep::Hunt;
                s.stow_wait = 5;
                s.we_opened = false;
                s.we_clicked = false;
            });
        }
        StowStep::Hunt => {
            let left = revise(|s| {
                s.stow_wait -= 1;
                s.stow_wait
            });
            if left > 0 {
                return;
            }
            if let Some(pos) = nearby_bin(game) {
                face_block(game, pos);
                revise(|s| {
                    s.chest = Some(pos);
                    s.stow_step = StowStep::Aim;
                    s.stow_wait = 10;
                    s.note = format!("Opening chest at {}, {}, {}", pos[0], pos[1], pos[2]);
                });
            } else {
                game.native_chat("Hookline: no chest or barrel nearby.");
                revise(|s| {
                    game.yaw = s.saved_yaw;
                    game.pitch = s.saved_pitch;
                    finish_stow(s);
                });
            }
        }
        StowStep::Aim => {
            let (left, pos) = revise(|s| {
                s.stow_wait -= 1;
                (s.stow_wait, s.chest)
            });
            if left > 0 {
                return;
            }
            if game.container.is_some() {
                game.native_chat("Hookline: a container is already open; leaving it alone.");
                revise(|s| {
                    game.yaw = s.saved_yaw;
                    game.pitch = s.saved_pitch;
                    finish_stow(s);
                });
                return;
            }
            if let Some(pos) = pos {
                click_bin(game, pos);
                revise(|s| {
                    s.we_clicked = true;
                    s.we_opened = false;
                    s.stow_step = StowStep::Transfer;
                    s.stow_wait = 40;
                    s.stow_index = 0;
                });
            } else {
                revise(finish_stow);
            }
        }
        StowStep::Transfer => {
            let (opened, clicked, stowing) = revise(|s| {
                if s.we_clicked && game.container.is_some() && !s.we_opened {
                    s.we_opened = true;
                }
                (s.we_opened, s.we_clicked, s.stowing)
            });
            if stow_may_click(stowing, opened, game.container.is_some()) {
                let wait = revise(|s| {
                    if s.stow_wait > 0 {
                        s.stow_wait -= 1;
                        true
                    } else {
                        false
                    }
                });
                if wait {
                    return;
                }
                let index = revise(|s| s.stow_index);
                if index < 36 {
                    let yes = should_stow_slot(game, index);
                    revise(|s| s.stow_index = index + 1);
                    if yes {
                        quick_stow(game, index);
                        revise(|s| s.stow_wait = 3);
                    }
                    return;
                }
                game.close_container();
                revise(|s| {
                    s.stow_step = StowStep::Recover;
                    s.stow_wait = 10;
                    s.we_opened = false;
                    s.we_clicked = false;
                });
            } else if clicked && game.container.is_none() {
                let left = revise(|s| {
                    s.stow_wait -= 1;
                    s.stow_wait
                });
                if left <= 0 {
                    game.native_chat("Hookline: could not open the chest.");
                    revise(|s| {
                        game.yaw = s.saved_yaw;
                        game.pitch = s.saved_pitch;
                        finish_stow(s);
                    });
                }
            } else {
                revise(|s| {
                    game.yaw = s.saved_yaw;
                    game.pitch = s.saved_pitch;
                    finish_stow(s);
                });
            }
        }
        StowStep::Recover => {
            let left = revise(|s| {
                s.stow_wait -= 1;
                s.stow_wait
            });
            if left <= 0 {
                revise(|s| {
                    game.yaw = s.saved_yaw;
                    game.pitch = s.saved_pitch;
                    finish_stow(s);
                });
            }
        }
    }
}

fn tick_lull(s: &mut Session) -> bool {
    if !s.prefs.running {
        s.lull = 0;
        return false;
    }
    if !s.prefs.rest_after_lull {
        return false;
    }
    if s.meal_phase.busy() {
        return false;
    }
    s.lull = s.lull.saturating_add(1);
    if s.lull < LULL_TICKS {
        return false;
    }
    halt(s);
    s.note = LULL_MSG.into();
    true
}

fn tick_wander(game: &mut crate::game::Game<'_>) {
    let nudge = revise(|s| {
        if !s.prefs.wander {
            s.wander = 0;
            return None;
        }
        s.wander += 1;
        if s.wander <= WANDER_PERIOD {
            return None;
        }
        s.wander = 0;
        Some(if mix64(&mut s.seed) % 2 == 0 { 5.0 } else { -5.0 })
    });
    if let Some(sign) = nudge {
        game.yaw += sign;
        if let Some(p) = game.player.as_mut() {
            p.yaw = game.yaw;
            if p.ground.on_ground {
                p.jumping = true;
            }
        }
    }
}

pub fn tick_stow_cool(s: &mut Session) {
    if !s.stowing && s.stow_cool > 0 {
        s.stow_cool -= 1;
    }
}

pub fn drive_tick(game: &mut crate::game::Game<'_>) {
    if let Some(msg) = leave_if_hurt(game) {
        game.native_chat(&msg);
        return;
    }
    let on = revise(|s| {
        tick_stow_cool(s);
        s.prefs.running
    });
    if !on {
        return;
    }
    revise(age_actions);
    if revise(tick_lull) {
        game.right = false;
        say(game, LULL_MSG);
        return;
    }
    revise(|s| {
        s.tally.ticks += 1;
        if s.settle > 0 {
            s.settle -= 1;
        }
    });
    if revise(|s| s.stowing) {
        run_stow(game);
        return;
    }
    tick_wander(game);
    if tick_meal(game) {
        return;
    }
    notice_bags(game);
    if free_slots(game) <= 1 {
        if game.hook.is_some() {
            if !revise(can_use) {
                return;
            }
            if let Some(rod) = held_rod(game) {
                let _ = press_use_rod(game, &rod);
            }
        }
        let yaw = game.yaw;
        let pitch = game.pitch;
        revise(|s| begin_stow(s, yaw, pitch));
        return;
    }
    if tick_reel(game) {
        return;
    }
    tick_recast(game);
}

pub fn on_splash(game: &mut crate::game::Game<'_>, packet: &horizon_client::mod_api::ClientboundPlayPacket) {
    use horizon_client::mod_api::ClientboundPlayPacket as C;
    if !game.native_on(MOD_ID) || !game.native_window() {
        return;
    }
    if !revise(|s| s.prefs.running) {
        return;
    }
    let splash_at = match packet {
        C::Sound(p) => {
            let Some(name) = game.remote_sound_name(&p.sound) else { return };
            if !splash_sound(&name) {
                return;
            }
            Some((f64::from(p.x) / 8.0, f64::from(p.y) / 8.0, f64::from(p.z) / 8.0))
        }
        C::SoundEntity(p) => {
            let Some(name) = game.remote_sound_name(&p.sound) else { return };
            if !splash_sound(&name) {
                return;
            }
            let Some(state) = game.remote_sound_entity_state(p.id) else { return };
            Some((state.position[0], state.position[1], state.position[2]))
        }
        _ => return,
    };
    let (hook_out, dist) = match &game.hook {
        Some(hook) => {
            let dist = splash_at.map(|(x, y, z)| range_sq(hook.position.x, hook.position.y, hook.position.z, x, y, z));
            (true, dist)
        }
        None => (false, splash_at.map(|_| f64::INFINITY)),
    };
    let cue = revise(|s| {
        let (cue, wait) = bite_from_splash(
            s.prefs.running,
            s.meal_phase.busy(),
            s.settle,
            s.reel_in > 0,
            hook_out,
            dist,
            s.prefs.pace,
            &mut s.seed,
        );
        if cue != BiteCue::Ignore {
            mark_activity(s);
        }
        if cue == BiteCue::Wait {
            s.reel_in = wait;
            s.note = format!("Bite: reeling in {wait} ticks");
        } else if cue == BiteCue::ReelNow {
            s.reel_in = 0;
        }
        cue
    });
    if cue == BiteCue::ReelNow {
        try_reel_now(game);
    }
}

pub fn leave_if_hurt(game: &mut crate::game::Game<'_>) -> Option<String> {
    let health = game.player.as_ref()?.health.health();
    let msg = revise(|s| {
        if s.prefs.running && s.prefs.bail_hurt && health < HURT_LEAVE {
            halt(s);
            let msg = format!("Hookline: leaving the server because health {health:.1} is below 3 hearts.");
            pulse(s, "Hookline left the server — health was too low");
            let dir = game.native_data_dir(MOD_ID);
            write_prefs(s, &dir);
            Some(msg)
        } else {
            None
        }
    })?;
    let _ = game.leave_server(Some(msg.clone()));
    Some(msg)
}

pub fn flip_from_key(game: &mut crate::game::Game<'_>) {
    let msg = revise(|s| {
        let msg = toggle_running(s);
        let dir = game.native_data_dir(MOD_ID);
        write_prefs(s, &dir);
        msg.to_string()
    });
    if msg == OFF_MSG {
        halt_in_world(game);
    }
    say(game, &msg);
}

pub fn handle_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    let cmd = command.trim();
    let (head, rest) = match cmd.split_once(' ') {
        Some((h, r)) => (h, r.trim()),
        None => (cmd, ""),
    };
    if head != "hookline" {
        return;
    }
    *ran = true;
    match rest {
        "" | "help" => {
            for line in help_text() {
                game.native_chat(&line);
            }
        }
        "panel" => {
            crate::game::hookline_panel::open(game);
            revise(|s| {
                s.tab = 0;
                s.scroll = 0.0;
                s.bar_grab = None;
            });
        }
        "toggle" => flip_from_key(game),
        "pace" => {
            let msg = revise(|s| {
                s.prefs.pace = !s.prefs.pace;
                let dir = game.native_data_dir(MOD_ID);
                write_prefs(s, &dir);
                if s.prefs.pace {
                    "Hookline: pace variance on"
                } else {
                    "Hookline: pace variance off"
                }
                .to_string()
            });
            game.native_chat(&msg);
        }
        "ledger" => {
            let line = revise(|s| {
                format!(
                    "Hookline ledger — fish {}, treasure {}, junk {}, hooks {}, casts {}, hook {}, treasure {}, xp ~{}",
                    s.tally.fish,
                    s.tally.treasure,
                    s.tally.junk,
                    s.tally.hooks,
                    s.tally.casts,
                    s.tally.hook_rate(),
                    s.tally.treasure_rate(),
                    s.tally.xp_guess
                )
            });
            game.native_chat(&line);
        }
        "clear-ledger" => {
            revise(|s| s.tally = SessionTally::default());
            game.native_chat("Hookline: ledger cleared.");
        }
        "toss" | "chime" => {
            let Some(stack) = game.inventory.get(game.selected).and_then(|s| s.clone()) else {
                game.native_chat("Hookline: hold an item first.");
                return;
            };
            let id = normalize_id(&stack.item);
            let chime = rest == "chime";
            let msg = revise(|s| {
                let set = if chime { &mut s.prefs.chime } else { &mut s.prefs.toss };
                let now = if set.contains(&id) {
                    set.remove(&id);
                    false
                } else {
                    set.insert(id.clone());
                    true
                };
                let dir = game.native_data_dir(MOD_ID);
                write_prefs(s, &dir);
                let verb = if chime { "chime" } else { "toss" };
                format!("Hookline: {verb} for {id} is {}", if now { "on" } else { "off" })
            });
            game.native_chat(&msg);
        }
        other => {
            game.native_chat(&format!("Hookline: unknown '{other}'. Try /hookline help."));
        }
    }
}

pub fn overlay_line() -> Option<String> {
    revise(|s| {
        if !s.prefs.overlay {
            return None;
        }
        if let Some((text, _)) = &s.flash {
            return Some(text.clone());
        }
        if !s.note.is_empty() {
            return Some(s.note.clone());
        }
        Some(if s.prefs.running { ON_MSG } else { OFF_MSG }.into())
    })
}

// silence an unused import in some builds
#[allow(dead_code)]
fn _unused() {
    let _ = (ballots_for, default_toss_set, is_fish, is_treasure, is_risky_food, splash_is_ours, MEAL_HOLD_TICKS);
}
