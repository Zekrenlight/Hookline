// Pure Hookline rules: catch policy, bite timing, meals, and config text.
// This file has no Horizon types so it can be tested on its own.

pub use catalog::*;
pub use detect::{bite_from_splash, splash_is_ours, splash_sound, BiteCue};
pub use ledger::SessionTally;
pub use meal::{
    hunger_needs_food, pick_food_origin, step_meal, FoodOrigin, MealAdvance, MealPhase, MealStep,
};
pub use persist::{dump_prefs, load_prefs, Prefs, FORMAT};
pub use policy::{
    ballots_for, charm_ballot, should_stow, should_toss, toss_from_ballots, Ballot, CHARM_ANY,
    CHARM_TOSS,
};
pub use timing::{
    jitter_wait, may_press_use, mix64, recast_pause, stretched_wait, ticks_until_use, BITE_REEL_MAX,
    BITE_REEL_MIN, LULL_TICKS, MEAL_HOLD_TICKS, MEAL_PRE_MAX, MEAL_PRE_MIN, MEAL_RECAST_MAX,
    MEAL_RECAST_MIN, MEAL_SWAP_BACK_MAX, MEAL_SWAP_BACK_MIN, MEAL_BETWEEN_MAX, MEAL_BETWEEN_MIN, USE_GAP,
};

pub mod catalog {
    pub const CATCH_ROWS: &[&str] = &[
        "minecraft:enchanted_book",
        "minecraft:fishing_rod",
        "minecraft:bow",
        "minecraft:salmon",
        "minecraft:cod",
        "minecraft:tropical_fish",
        "minecraft:pufferfish",
        "minecraft:name_tag",
        "minecraft:stick",
        "minecraft:nautilus_shell",
        "minecraft:leather_boots",
        "minecraft:potion",
        "minecraft:bone",
        "minecraft:lily_pad",
        "minecraft:saddle",
        "minecraft:ink_sac",
        "minecraft:bowl",
        "minecraft:string",
        "minecraft:tripwire_hook",
        "minecraft:rotten_flesh",
        "minecraft:leather",
    ];

    pub const DEFAULT_TOSS: &[&str] = &[
        "minecraft:lily_pad",
        "minecraft:bowl",
        "minecraft:leather",
        "minecraft:leather_boots",
        "minecraft:rotten_flesh",
        "minecraft:stick",
        "minecraft:string",
        "minecraft:bone",
        "minecraft:ink_sac",
        "minecraft:tripwire_hook",
        "minecraft:potion",
    ];

    pub const FISH: &[&str] = &[
        "minecraft:cod",
        "minecraft:salmon",
        "minecraft:pufferfish",
        "minecraft:tropical_fish",
    ];

    pub const TREASURE: &[&str] = &[
        "minecraft:enchanted_book",
        "minecraft:bow",
        "minecraft:fishing_rod",
        "minecraft:name_tag",
        "minecraft:saddle",
        "minecraft:nautilus_shell",
    ];

    pub const RISKY_FOOD: &[&str] = &[
        "minecraft:rotten_flesh",
        "minecraft:spider_eye",
        "minecraft:pufferfish",
        "minecraft:poisonous_potato",
        "minecraft:suspicious_stew",
    ];

    /// (label, vanilla max, default floor). -1 toss, 0 any, 1..=max minimum.
    pub const CHARMS: &[(&str, i32, i32)] = &[
        ("Mending", 1, 1),
        ("Unbreaking", 3, 3),
        ("Luck of the Sea", 3, 0),
        ("Lure", 3, 0),
        ("Protection", 4, 0),
        ("Sharpness", 5, 0),
        ("Efficiency", 5, 0),
        ("Power", 5, 0),
        ("Punch", 2, 0),
        ("Flame", 1, 0),
        ("Infinity", 1, 0),
        ("Silk Touch", 1, 0),
        ("Fortune", 3, 0),
        ("Feather Falling", 4, 0),
        ("Depth Strider", 3, 0),
        ("Respiration", 3, 0),
        ("Aqua Affinity", 1, 0),
        ("Thorns", 3, 0),
        ("Fire Protection", 4, 0),
        ("Blast Protection", 4, 0),
        ("Projectile Protection", 4, 0),
        ("Smite", 5, 0),
        ("Bane of Arthropods", 5, 0),
        ("Knockback", 2, 0),
        ("Fire Aspect", 2, 0),
        ("Looting", 3, 0),
        ("Sweeping Edge", 3, 0),
        ("Channeling", 1, 0),
        ("Impaling", 5, 0),
        ("Loyalty", 3, 0),
        ("Riptide", 3, 0),
        ("Multishot", 1, 0),
        ("Piercing", 4, 0),
        ("Quick Charge", 3, 0),
        ("Soul Speed", 3, 0),
        ("Swift Sneak", 3, 0),
        ("Density", 5, 0),
        ("Breach", 4, 0),
        ("Wind Burst", 3, 0),
    ];

    pub const CHARM_GEAR: &[&str] = &[
        "minecraft:enchanted_book",
        "minecraft:fishing_rod",
        "minecraft:bow",
    ];

    pub fn normalize_id(raw: &str) -> String {
        let t = raw.trim().to_ascii_lowercase().replace(' ', "_");
        if t.is_empty() {
            return t;
        }
        if t.contains(':') {
            t
        } else {
            format!("minecraft:{t}")
        }
    }

    pub fn label_of(id: &str) -> String {
        let n = normalize_id(id);
        let path = n.rsplit_once(':').map(|(_, p)| p).unwrap_or(&n);
        path.split('_')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let mut c = p.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn is_rod(item: &str) -> bool {
        let id = normalize_id(item);
        id == "minecraft:fishing_rod" || id.ends_with(":fishing_rod")
    }

    fn pack_treasure(id: &str) -> bool {
        id.starts_with("gofish:") && (id.contains("crate") || id.contains("rod"))
    }

    fn pack_fish(id: &str) -> bool {
        id.starts_with("gofish:") && !pack_treasure(id)
    }

    pub fn is_fish(item: &str) -> bool {
        let id = normalize_id(item);
        FISH.contains(&id.as_str()) || pack_fish(&id)
    }

    pub fn is_treasure(item: &str) -> bool {
        let id = normalize_id(item);
        TREASURE.contains(&id.as_str()) || pack_treasure(&id)
    }

    pub fn is_catch_item(item: &str) -> bool {
        let id = normalize_id(item);
        CATCH_ROWS.contains(&id.as_str()) || pack_fish(&id) || pack_treasure(&id)
    }

    pub fn is_charm_gear(item: &str) -> bool {
        CHARM_GEAR.contains(&normalize_id(item).as_str())
    }

    pub fn is_risky_food(item: &str) -> bool {
        RISKY_FOOD.contains(&normalize_id(item).as_str())
    }

    pub fn charm_slug(label: &str) -> String {
        normalize_id(label)
    }

    pub fn charm_matches(label_or_id: &str, other: &str) -> bool {
        charm_slug(label_or_id) == charm_slug(other)
    }

    pub fn charm_max(name: &str) -> i32 {
        CHARMS
            .iter()
            .find(|(n, _, _)| charm_matches(n, name))
            .map(|(_, m, _)| *m)
            .unwrap_or(1)
    }

    pub fn known_charm(name: &str) -> bool {
        CHARMS.iter().any(|(n, _, _)| charm_matches(n, name))
    }

    pub fn default_charm_floors() -> std::collections::HashMap<String, i32> {
        CHARMS
            .iter()
            .map(|(n, _, d)| (charm_slug(n), *d))
            .collect()
    }

    pub fn default_toss_set() -> std::collections::HashSet<String> {
        DEFAULT_TOSS.iter().map(|s| (*s).to_string()).collect()
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CatchKind {
        Fish,
        Treasure,
        Junk,
    }

    pub fn sort_catch(item: &str) -> CatchKind {
        if is_treasure(item) {
            CatchKind::Treasure
        } else if is_fish(item) {
            CatchKind::Fish
        } else {
            CatchKind::Junk
        }
    }

    pub fn rod_is_worn(damage: u32, max_damage: u32) -> bool {
        max_damage > 0 && max_damage.saturating_sub(damage) <= 2
    }
}

pub mod timing {
    pub const BITE_REEL_MIN: i32 = 5;
    pub const BITE_REEL_MAX: i32 = 14;
    pub const RECAST_MIN: i32 = 15;
    pub const RECAST_MAX: i32 = 24;
    pub const SETTLE_TICKS: i32 = 30;
    pub const LULL_TICKS: i32 = 6000;
    pub const MEAL_HOLD_TICKS: i32 = 40;
    pub const MEAL_PRE_MIN: i32 = 15;
    pub const MEAL_PRE_MAX: i32 = 30;
    pub const MEAL_AFTER_SWAP_MIN: i32 = 2;
    pub const MEAL_AFTER_SWAP_MAX: i32 = 5;
    pub const MEAL_SWAP_BACK_MIN: i32 = 30;
    pub const MEAL_SWAP_BACK_MAX: i32 = 70;
    pub const MEAL_RECAST_MIN: i32 = 40;
    pub const MEAL_RECAST_MAX: i32 = 100;
    pub const MEAL_BETWEEN_MIN: i32 = 15;
    pub const MEAL_BETWEEN_MAX: i32 = 35;
    pub const USE_GAP: i32 = 2;
    pub const HURT_LEAVE: f32 = 6.0;
    pub const HUNGER_FULL: i32 = 20;
    pub const WANDER_PERIOD: i32 = 1200;
    pub const STOW_COOLDOWN: i32 = 200;
    pub const NO_FOOD_EVERY: i32 = 200;

    pub fn mix64(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = *seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    pub fn roll(seed: &mut u64, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        mix64(seed) % n
    }

    /// Average of two independent rolls in `[lo, hi]`, so waits are not a flat repeat.
    pub fn jitter_wait(seed: &mut u64, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        let a = roll(seed, span);
        let b = roll(seed, span);
        lo + ((a + b) / 2) as i32
    }

    pub fn stretched_wait(seed: &mut u64, lo: i32, hi: i32, pace: bool) -> i32 {
        let mut d = jitter_wait(seed, lo, hi);
        if pace {
            d += jitter_wait(seed, 3, 10);
        }
        d.max(1)
    }

    pub fn bite_reel_pause(seed: &mut u64) -> i32 {
        jitter_wait(seed, BITE_REEL_MIN, BITE_REEL_MAX)
    }

    pub fn recast_pause(seed: &mut u64, pace: bool) -> i32 {
        stretched_wait(seed, RECAST_MIN, RECAST_MAX, pace)
    }

    pub fn meal_pre_pause(seed: &mut u64, pace: bool) -> i32 {
        stretched_wait(seed, MEAL_PRE_MIN, MEAL_PRE_MAX, pace)
    }

    pub fn meal_after_swap_pause(seed: &mut u64, pace: bool) -> i32 {
        stretched_wait(seed, MEAL_AFTER_SWAP_MIN, MEAL_AFTER_SWAP_MAX, pace)
    }

    pub fn meal_between_pause(seed: &mut u64, pace: bool) -> i32 {
        stretched_wait(seed, MEAL_BETWEEN_MIN, MEAL_BETWEEN_MAX, pace)
    }

    pub fn meal_swap_back_pause(seed: &mut u64, pace: bool) -> i32 {
        stretched_wait(seed, MEAL_SWAP_BACK_MIN, MEAL_SWAP_BACK_MAX, pace)
    }

    pub fn meal_recast_pause(seed: &mut u64, pace: bool) -> i32 {
        stretched_wait(seed, MEAL_RECAST_MIN, MEAL_RECAST_MAX, pace)
    }

    pub fn may_press_use(since_release: i32, since_slot: i32, since_throw: i32) -> bool {
        since_release >= USE_GAP && since_slot >= USE_GAP && since_throw >= USE_GAP
    }

    pub fn ticks_until_use(since_release: i32, since_slot: i32, since_throw: i32) -> i32 {
        let a = (USE_GAP - since_release).max(0);
        let b = (USE_GAP - since_slot).max(0);
        let c = (USE_GAP - since_throw).max(0);
        a.max(b).max(c)
    }
}

pub mod detect {
    use super::timing::{bite_reel_pause, SETTLE_TICKS};

    pub const SPLASH: &str = "minecraft:entity.fishing_bobber.splash";
    const RANGE_SQ: f64 = 4.0;

    pub fn splash_sound(name: &str) -> bool {
        let n = name.trim();
        n == SPLASH || n == "entity.fishing_bobber.splash"
    }

    pub fn splash_is_ours(hook_out: bool, dist_sq: Option<f64>) -> bool {
        if !hook_out {
            return false;
        }
        match dist_sq {
            None => false,
            Some(d) => d < RANGE_SQ,
        }
    }

    pub fn range_sq(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> f64 {
        let dx = ax - bx;
        let dy = ay - by;
        let dz = az - bz;
        dx * dx + dy * dy + dz * dz
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BiteCue {
        Ignore,
        Wait,
        ReelNow,
    }

    pub fn bite_from_splash(
        running: bool,
        meal_busy: bool,
        settle: i32,
        reel_armed: bool,
        hook_out: bool,
        dist_sq: Option<f64>,
        pace: bool,
        seed: &mut u64,
    ) -> (BiteCue, i32) {
        if !running || meal_busy {
            return (BiteCue::Ignore, 0);
        }
        if settle > 0 || reel_armed {
            return (BiteCue::Ignore, 0);
        }
        if !splash_is_ours(hook_out, dist_sq) {
            return (BiteCue::Ignore, 0);
        }
        if pace {
            let wait = bite_reel_pause(seed);
            (BiteCue::Wait, wait)
        } else {
            (BiteCue::ReelNow, 0)
        }
    }

    pub fn settle_after_cast() -> i32 {
        SETTLE_TICKS
    }
}

pub mod policy {
    use super::catalog::{
        charm_matches, charm_max, is_catch_item, is_charm_gear, is_rod, known_charm, normalize_id,
    };
    use std::collections::{HashMap, HashSet};

    pub const CHARM_TOSS: i32 = -1;
    pub const CHARM_ANY: i32 = 0;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Ballot {
        pub name: String,
        pub keep: bool,
        pub level: i32,
        pub max: i32,
    }

    impl Ballot {
        fn at_cap(&self) -> bool {
            self.max > 0 && self.level >= self.max
        }
        fn closeness(&self) -> f64 {
            if self.max <= 0 {
                0.0
            } else {
                self.level as f64 / self.max as f64
            }
        }
    }

    pub fn floor_meets(level: i32, floor: i32) -> bool {
        if floor <= CHARM_ANY {
            level >= 1
        } else {
            level >= floor
        }
    }

    pub fn charm_ballot(name: &str, level: i32, floor: i32) -> Ballot {
        let max = charm_max(name);
        let keep = floor >= CHARM_ANY && floor_meets(level, floor);
        Ballot {
            name: name.to_string(),
            keep,
            level,
            max,
        }
    }

    pub fn ballots_for(
        charms: &[(String, i32)],
        floors: &HashMap<String, i32>,
    ) -> Vec<Ballot> {
        let mut out = Vec::new();
        for (id, level) in charms {
            let known = known_charm(id);
            let floor = floors.iter().find(|(k, _)| charm_matches(k, id)).map(|(_, v)| *v);
            if !known && floor.is_none() {
                continue;
            }
            out.push(charm_ballot(id, *level, floor.unwrap_or(CHARM_ANY)));
        }
        out
    }

    fn stronger(a: &Ballot, b: &Ballot) -> std::cmp::Ordering {
        a.at_cap()
            .cmp(&b.at_cap())
            .then_with(|| a.closeness().partial_cmp(&b.closeness()).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.level.cmp(&b.level))
    }

    pub fn toss_from_ballots(votes: &[Ballot]) -> bool {
        if votes.is_empty() {
            return false;
        }
        if votes.iter().all(|v| v.keep) {
            return false;
        }
        if votes.iter().all(|v| !v.keep) {
            return true;
        }
        if let Some(mending) = votes.iter().find(|v| charm_matches(&v.name, "mending")) {
            return !mending.keep;
        }
        let mut best = &votes[0];
        let mut tied = false;
        for v in &votes[1..] {
            match stronger(v, best) {
                std::cmp::Ordering::Greater => {
                    best = v;
                    tied = false;
                }
                std::cmp::Ordering::Equal => {
                    if v.keep != best.keep {
                        tied = true;
                    }
                }
                std::cmp::Ordering::Less => {}
            }
        }
        if tied {
            return false;
        }
        !best.keep
    }

    fn listed_toss(toss: &HashSet<String>, item: &str) -> bool {
        let id = normalize_id(item);
        toss.iter().any(|t| normalize_id(t) == id)
    }

    pub fn should_toss(
        item: &str,
        toss: &HashSet<String>,
        charms: &[(String, i32)],
        floors: &HashMap<String, i32>,
        rod_in_use: bool,
    ) -> bool {
        if rod_in_use {
            return false;
        }
        let id = normalize_id(item);
        if listed_toss(toss, &id) {
            return true;
        }
        if !is_charm_gear(&id) {
            return false;
        }
        toss_from_ballots(&ballots_for(charms, floors))
    }

    pub fn should_stow(
        item: &str,
        toss: &HashSet<String>,
        charms: &[(String, i32)],
        floors: &HashMap<String, i32>,
    ) -> bool {
        let id = normalize_id(item);
        if id == "minecraft:air" || is_rod(&id) {
            return false;
        }
        if !is_catch_item(&id) {
            return false;
        }
        !should_toss(&id, toss, charms, floors, false)
    }

    pub fn may_begin_stow(already: bool, cooldown: i32) -> bool {
        !already && cooldown <= 0
    }

    pub fn stow_may_click(stowing: bool, we_opened: bool, container_open: bool) -> bool {
        stowing && we_opened && container_open
    }
}

pub mod meal {
    use super::catalog::{is_risky_food, normalize_id};
    use super::timing::{may_press_use, MEAL_AFTER_SWAP_MIN, MEAL_HOLD_TICKS, USE_GAP, HUNGER_FULL};

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum MealPhase {
        #[default]
        Rest,
        AfterReel,
        BeforeUse,
        Chewing,
        BetweenBites,
        BeforeRod,
        BeforeCast,
    }

    impl MealPhase {
        pub fn busy(self) -> bool {
            self != MealPhase::Rest
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MealStep {
        Idle,
        Reel,
        SwapFood,
        StartChew,
        KeepChew,
        LetGo,
        SwapRod,
        CastAgain,
        Cancel,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct MealAdvance {
        pub phase: MealPhase,
        pub timer: i32,
        pub step: MealStep,
    }

    pub fn hunger_needs_food(food: i32, _health: f32, _max_health: f32) -> bool {
        food < HUNGER_FULL
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum FoodOrigin {
        Offhand,
        Hotbar { slot: usize },
    }

    impl FoodOrigin {
        pub fn hand(&self) -> i32 {
            match self {
                FoodOrigin::Offhand => 1,
                FoodOrigin::Hotbar { .. } => 0,
            }
        }
        pub fn needs_swap(&self, selected: usize) -> bool {
            match self {
                FoodOrigin::Offhand => false,
                FoodOrigin::Hotbar { slot } => *slot != selected,
            }
        }
        pub fn chew_slot(&self) -> i32 {
            match self {
                FoodOrigin::Offhand => 40,
                FoodOrigin::Hotbar { slot } => *slot as i32,
            }
        }
    }

    /// Off-hand first, then hotbar only (the held main-hand slot counts as hotbar).
    /// Skip risky food while a safer one exists. Main-inventory food is not used.
    pub fn pick_food_origin(
        offhand: Option<&str>,
        hotbar: &[Option<String>; 9],
        selected: usize,
    ) -> Option<FoodOrigin> {
        let mut safe = Vec::new();
        let mut risky = Vec::new();
        let push = |id: &str, origin: FoodOrigin, safe: &mut Vec<FoodOrigin>, risky: &mut Vec<FoodOrigin>| {
            if is_risky_food(id) {
                risky.push(origin);
            } else {
                safe.push(origin);
            }
        };
        if let Some(id) = offhand {
            if !normalize_id(id).is_empty() {
                push(id, FoodOrigin::Offhand, &mut safe, &mut risky);
            }
        }
        if let Some(Some(id)) = hotbar.get(selected) {
            push(id, FoodOrigin::Hotbar { slot: selected }, &mut safe, &mut risky);
        }
        for (slot, id) in hotbar.iter().enumerate() {
            if slot == selected {
                continue;
            }
            if let Some(id) = id {
                push(id, FoodOrigin::Hotbar { slot }, &mut safe, &mut risky);
            }
        }
        safe.into_iter().next().or_else(|| risky.into_iter().next())
    }

    pub fn step_meal(
        phase: MealPhase,
        timer: i32,
        meals: bool,
        hungry: bool,
        has_food: bool,
        hook_out: bool,
        player_use: bool,
        still_on_food: bool,
        can_use: bool,
        need_swap: bool,
        pre_wait: i32,
        after_swap: i32,
        between_wait: i32,
        back_wait: i32,
        recast_wait: i32,
    ) -> MealAdvance {
        let cancel = MealAdvance {
            phase: MealPhase::Rest,
            timer: 0,
            step: MealStep::Cancel,
        };
        match phase {
            MealPhase::Rest => {
                if !meals || !hungry || !has_food || player_use {
                    return MealAdvance {
                        phase: MealPhase::Rest,
                        timer: 0,
                        step: MealStep::Idle,
                    };
                }
                if hook_out {
                    if !can_use {
                        return MealAdvance {
                            phase: MealPhase::Rest,
                            timer: 0,
                            step: MealStep::Idle,
                        };
                    }
                    return MealAdvance {
                        phase: MealPhase::AfterReel,
                        timer: pre_wait.max(1),
                        step: MealStep::Reel,
                    };
                }
                if need_swap {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: after_swap.max(USE_GAP),
                        step: MealStep::SwapFood,
                    };
                }
                if !can_use {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: 1,
                        step: MealStep::Idle,
                    };
                }
                MealAdvance {
                    phase: MealPhase::Chewing,
                    timer: MEAL_HOLD_TICKS,
                    step: MealStep::StartChew,
                }
            }
            MealPhase::AfterReel => {
                if !meals || !has_food {
                    return cancel;
                }
                if timer > 1 {
                    return MealAdvance {
                        phase: MealPhase::AfterReel,
                        timer: timer - 1,
                        step: MealStep::Idle,
                    };
                }
                if need_swap {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: after_swap.max(MEAL_AFTER_SWAP_MIN),
                        step: MealStep::SwapFood,
                    };
                }
                if !can_use {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: 1,
                        step: MealStep::Idle,
                    };
                }
                MealAdvance {
                    phase: MealPhase::Chewing,
                    timer: MEAL_HOLD_TICKS,
                    step: MealStep::StartChew,
                }
            }
            MealPhase::BeforeUse => {
                if !meals || !has_food || !still_on_food {
                    return cancel;
                }
                if timer > 1 {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: timer - 1,
                        step: MealStep::Idle,
                    };
                }
                if !can_use {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: 1,
                        step: MealStep::Idle,
                    };
                }
                MealAdvance {
                    phase: MealPhase::Chewing,
                    timer: MEAL_HOLD_TICKS,
                    step: MealStep::StartChew,
                }
            }
            MealPhase::Chewing => {
                if !meals || !still_on_food {
                    return cancel;
                }
                if timer > 1 {
                    return MealAdvance {
                        phase: MealPhase::Chewing,
                        timer: timer - 1,
                        step: MealStep::KeepChew,
                    };
                }
                if hungry && has_food {
                    return MealAdvance {
                        phase: MealPhase::BetweenBites,
                        timer: between_wait.max(USE_GAP),
                        step: MealStep::LetGo,
                    };
                }
                MealAdvance {
                    phase: MealPhase::BeforeRod,
                    timer: back_wait.max(1),
                    step: MealStep::LetGo,
                }
            }
            MealPhase::BetweenBites => {
                if !meals {
                    return cancel;
                }
                if !hungry || !has_food {
                    return MealAdvance {
                        phase: MealPhase::BeforeRod,
                        timer: back_wait.max(1),
                        step: MealStep::Idle,
                    };
                }
                if timer > 1 {
                    return MealAdvance {
                        phase: MealPhase::BetweenBites,
                        timer: timer - 1,
                        step: MealStep::Idle,
                    };
                }
                if need_swap {
                    return MealAdvance {
                        phase: MealPhase::BeforeUse,
                        timer: after_swap.max(MEAL_AFTER_SWAP_MIN),
                        step: MealStep::SwapFood,
                    };
                }
                if !can_use {
                    return MealAdvance {
                        phase: MealPhase::BetweenBites,
                        timer: 1,
                        step: MealStep::Idle,
                    };
                }
                MealAdvance {
                    phase: MealPhase::Chewing,
                    timer: MEAL_HOLD_TICKS,
                    step: MealStep::StartChew,
                }
            }
            MealPhase::BeforeRod => {
                if timer > 1 {
                    return MealAdvance {
                        phase: MealPhase::BeforeRod,
                        timer: timer - 1,
                        step: MealStep::Idle,
                    };
                }
                MealAdvance {
                    phase: MealPhase::BeforeCast,
                    timer: recast_wait.max(1),
                    step: MealStep::SwapRod,
                }
            }
            MealPhase::BeforeCast => {
                if timer > 1 {
                    return MealAdvance {
                        phase: MealPhase::BeforeCast,
                        timer: timer - 1,
                        step: MealStep::Idle,
                    };
                }
                if !can_use {
                    return MealAdvance {
                        phase: MealPhase::BeforeCast,
                        timer: 1,
                        step: MealStep::Idle,
                    };
                }
                MealAdvance {
                    phase: MealPhase::Rest,
                    timer: 0,
                    step: MealStep::CastAgain,
                }
            }
        }
    }

    pub fn use_is_allowed(since_release: i32, since_slot: i32, since_throw: i32) -> bool {
        may_press_use(since_release, since_slot, since_throw)
    }
}

pub mod persist {
    use super::catalog::{charm_slug, default_charm_floors, default_toss_set, normalize_id};
    use super::policy::{CHARM_ANY, CHARM_TOSS};
    use std::collections::{HashMap, HashSet};

    pub const FORMAT: u32 = 1;
    pub const FILE_NAME: &str = "hookline.cfg";
    pub const TOGGLE_KEY: &str = "key.keyboard.v";
    pub const PANEL_KEY: &str = "key.keyboard.o";

    #[derive(Clone, Debug)]
    pub struct Prefs {
        pub format: u32,
        pub running: bool,
        pub pace: bool,
        pub wander: bool,
        pub meals: bool,
        pub worn_rod: bool,
        pub bail_hurt: bool,
        pub rest_after_lull: bool,
        pub overlay: bool,
        pub toss: HashSet<String>,
        pub chime: HashSet<String>,
        pub charm_floor: HashMap<String, i32>,
        pub toggle_key: String,
        pub panel_key: String,
    }

    impl Default for Prefs {
        fn default() -> Self {
            Self {
                format: FORMAT,
                running: false,
                pace: true,
                wander: false,
                meals: false,
                worn_rod: false,
                bail_hurt: true,
                rest_after_lull: true,
                overlay: false,
                toss: default_toss_set(),
                chime: HashSet::new(),
                charm_floor: default_charm_floors(),
                toggle_key: TOGGLE_KEY.into(),
                panel_key: PANEL_KEY.into(),
            }
        }
    }

    fn flag(v: bool) -> &'static str {
        if v {
            "yes"
        } else {
            "no"
        }
    }

    fn parse_flag(v: &str) -> bool {
        matches!(v.trim().to_ascii_lowercase().as_str(), "yes" | "true" | "on" | "1")
    }

    fn join_ids(ids: &HashSet<String>) -> String {
        let mut v: Vec<_> = ids.iter().cloned().collect();
        v.sort();
        v.join(",")
    }

    fn split_ids(raw: &str) -> HashSet<String> {
        raw.split(',')
            .map(normalize_id)
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn join_charms(map: &HashMap<String, i32>) -> String {
        let mut v: Vec<_> = map.iter().map(|(k, v)| format!("{k}:{v}")).collect();
        v.sort();
        v.join(",")
    }

    fn split_charms(raw: &str) -> HashMap<String, i32> {
        let mut out = HashMap::new();
        for part in raw.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some((k, v)) = part.rsplit_once(':') {
                if let Ok(n) = v.trim().parse::<i32>() {
                    let n = n.clamp(CHARM_TOSS, 10);
                    out.insert(charm_slug(k), n);
                }
            }
        }
        out
    }

    pub fn dump_prefs(p: &Prefs) -> String {
        let mut s = String::new();
        s.push_str(&format!("format={}\n", p.format.max(FORMAT)));
        s.push_str(&format!("running={}\n", flag(p.running)));
        s.push_str(&format!("pace={}\n", flag(p.pace)));
        s.push_str(&format!("wander={}\n", flag(p.wander)));
        s.push_str(&format!("meals={}\n", flag(p.meals)));
        s.push_str(&format!("worn_rod={}\n", flag(p.worn_rod)));
        s.push_str(&format!("bail_hurt={}\n", flag(p.bail_hurt)));
        s.push_str(&format!("rest_after_lull={}\n", flag(p.rest_after_lull)));
        s.push_str(&format!("overlay={}\n", flag(p.overlay)));
        s.push_str(&format!("toss={}\n", join_ids(&p.toss)));
        s.push_str(&format!("chime={}\n", join_ids(&p.chime)));
        s.push_str(&format!("charm={}\n", join_charms(&p.charm_floor)));
        s.push_str(&format!("toggle_key={}\n", p.toggle_key));
        s.push_str(&format!("panel_key={}\n", p.panel_key));
        s
    }

    pub fn load_prefs(text: &str) -> (Prefs, std::collections::HashSet<String>) {
        let mut p = Prefs::default();
        let mut seen = std::collections::HashSet::new();
        let mut saw_toss = false;
        let mut saw_charm = false;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let v = v.trim();
            seen.insert(k.to_string());
            match k {
                "format" => {
                    if let Ok(n) = v.parse() {
                        p.format = n;
                    }
                }
                "running" => p.running = parse_flag(v),
                "pace" => p.pace = parse_flag(v),
                "wander" => p.wander = parse_flag(v),
                "meals" => p.meals = parse_flag(v),
                "worn_rod" => p.worn_rod = parse_flag(v),
                "bail_hurt" => p.bail_hurt = parse_flag(v),
                "rest_after_lull" => p.rest_after_lull = parse_flag(v),
                "overlay" => p.overlay = parse_flag(v),
                "toss" => {
                    saw_toss = true;
                    p.toss = split_ids(v);
                }
                "chime" => p.chime = split_ids(v),
                "charm" => {
                    saw_charm = true;
                    let mut floors = default_charm_floors();
                    for (k, n) in split_charms(v) {
                        floors.insert(k, n);
                    }
                    p.charm_floor = floors;
                }
                "toggle_key" => {
                    if !v.is_empty() {
                        p.toggle_key = v.to_string();
                    }
                }
                "panel_key" => {
                    if !v.is_empty() {
                        p.panel_key = v.to_string();
                    }
                }
                _ => {}
            }
        }
        if !saw_toss {
            p.toss = default_toss_set();
        }
        if !saw_charm {
            p.charm_floor = default_charm_floors();
        }
        // Keep Any as 0 if someone wrote a bare charm with no floor.
        for v in p.charm_floor.values_mut() {
            if *v < CHARM_TOSS {
                *v = CHARM_ANY;
            }
        }
        (p, seen)
    }

    pub fn apply_native_overlay(p: &mut Prefs, native: &HashMap<String, String>, file_keys: &HashSet<String>) {
        let put = |key: &str, slot: &mut bool| {
            if file_keys.contains(key) {
                return;
            }
            if let Some(v) = native.get(key) {
                *slot = parse_flag(v);
            }
        };
        put("pace", &mut p.pace);
        put("wander", &mut p.wander);
        put("meals", &mut p.meals);
        put("worn_rod", &mut p.worn_rod);
        put("bail_hurt", &mut p.bail_hurt);
        put("rest_after_lull", &mut p.rest_after_lull);
        put("overlay", &mut p.overlay);
    }

    pub fn cycle_charm_floor(current: i32, max: i32) -> i32 {
        if current == CHARM_TOSS {
            CHARM_ANY
        } else if current == CHARM_ANY {
            1.min(max.max(1))
        } else if current >= max {
            CHARM_TOSS
        } else {
            current + 1
        }
    }
}

pub mod ledger {
    #[derive(Clone, Debug, Default)]
    pub struct SessionTally {
        pub fish: u32,
        pub treasure: u32,
        pub junk: u32,
        pub casts: u32,
        pub hooks: u32,
        pub ticks: u64,
        pub xp_guess: u32,
    }

    impl SessionTally {
        pub fn hook_rate(&self) -> String {
            if self.casts == 0 {
                return "0%".into();
            }
            let pct = (self.hooks as f64 / self.casts as f64 * 100.0) as u32;
            format!("{pct}%")
        }

        pub fn treasure_rate(&self) -> String {
            if self.casts == 0 {
                return "0%".into();
            }
            let pct = (self.treasure as f64 / self.casts as f64 * 100.0) as u32;
            format!("{pct}%")
        }
    }
}


// List layout for the Loot and Enchants pages. Kept here so panel math can be
// tested without the game renderer. Row positions are in pixels; the last row
// must be able to sit fully inside the viewport at max scroll.
pub mod listing {
    pub const ROW: f32 = 22.0;
    pub const BAR_W: f32 = 6.0;
    pub const BAR_GAP: f32 = 2.0;
    pub const THUMB_MIN: f32 = 16.0;
    /// Search field (16) plus gap before the first loot row.
    pub const LOOT_HEADER: f32 = 20.0;
    /// Caption plus gap before the first enchant row.
    pub const CHARM_HEADER: f32 = 16.0;
    /// Panel height is clamped to this range; the body is this minus 60.
    pub const PANEL_H_MIN: f32 = 200.0;
    pub const PANEL_H_MAX: f32 = 320.0;
    pub const BODY_INSET: f32 = 60.0;

    pub fn body_h(panel_h: f32) -> f32 {
        panel_h.clamp(PANEL_H_MIN, PANEL_H_MAX) - BODY_INSET
    }

    pub fn loot_view_h(body_h: f32) -> f32 {
        (body_h - LOOT_HEADER).max(0.0)
    }

    pub fn charm_view_h(body_h: f32) -> f32 {
        (body_h - CHARM_HEADER).max(0.0)
    }

    pub fn content_h(total: usize) -> f32 {
        total as f32 * ROW
    }

    pub fn max_scroll(total: usize, view_h: f32) -> f32 {
        (content_h(total) - view_h).max(0.0)
    }

    pub fn clamp_scroll(scroll: f32, total: usize, view_h: f32) -> f32 {
        if !scroll.is_finite() {
            return 0.0;
        }
        scroll.clamp(0.0, max_scroll(total, view_h))
    }

    /// Fully visible rows at this offset: `start..end`.
    pub fn shown_rows(scroll: f32, total: usize, view_h: f32) -> (usize, usize) {
        if total == 0 || view_h < ROW {
            return (0, 0);
        }
        let scroll = clamp_scroll(scroll, total, view_h);
        let top = scroll - 0.01;
        let bot = scroll + view_h + 0.01;
        let mut start = None;
        let mut end = 0;
        for i in 0..total {
            let y0 = i as f32 * ROW;
            let y1 = y0 + ROW;
            if y0 >= top && y1 <= bot {
                if start.is_none() {
                    start = Some(i);
                }
                end = i + 1;
            }
        }
        (start.unwrap_or(0), end)
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Thumb {
        pub y: f32,
        pub h: f32,
    }

    /// Thumb position is relative to the track top. `None` when the list fits.
    pub fn thumb(scroll: f32, total: usize, view_h: f32, track_h: f32) -> Option<Thumb> {
        if total == 0 || track_h <= 0.0 || content_h(total) <= view_h + 0.01 {
            return None;
        }
        let thumb_h = (view_h / content_h(total) * track_h).clamp(THUMB_MIN, track_h);
        let max = max_scroll(total, view_h);
        let scroll = clamp_scroll(scroll, total, view_h);
        let travel = (track_h - thumb_h).max(0.0);
        let y = if max <= 0.0 { 0.0 } else { (scroll / max) * travel };
        Some(Thumb { y, h: thumb_h })
    }

    /// Jump so the thumb is centered on a click along the track.
    pub fn scroll_at_track(local_y: f32, total: usize, view_h: f32, track_h: f32) -> f32 {
        let Some(t) = thumb(0.0, total, view_h, track_h) else {
            return 0.0;
        };
        let travel = (track_h - t.h).max(0.0);
        if travel <= 0.0 {
            return 0.0;
        }
        let y = (local_y - t.h * 0.5).clamp(0.0, travel);
        (y / travel) * max_scroll(total, view_h)
    }

    pub fn scroll_from_drag(
        local_y: f32,
        grab: f32,
        total: usize,
        view_h: f32,
        track_h: f32,
    ) -> f32 {
        let Some(t) = thumb(0.0, total, view_h, track_h) else {
            return 0.0;
        };
        let travel = (track_h - t.h).max(0.0);
        if travel <= 0.0 {
            return 0.0;
        }
        let y = (local_y - grab).clamp(0.0, travel);
        (y / travel) * max_scroll(total, view_h)
    }

    pub fn row_top(index: usize, scroll: f32) -> f32 {
        index as f32 * ROW - scroll
    }
}
