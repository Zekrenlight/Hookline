use hookline_logic::*;
use std::collections::{HashMap, HashSet};

#[test]
fn catch_rows_start_with_our_order() {
    assert_eq!(
        &CATCH_ROWS[..7],
        &[
            "minecraft:enchanted_book",
            "minecraft:fishing_rod",
            "minecraft:bow",
            "minecraft:salmon",
            "minecraft:cod",
            "minecraft:tropical_fish",
            "minecraft:pufferfish",
        ]
    );
    assert_eq!(CATCH_ROWS.len(), 21);
}

#[test]
fn ids_normalize_and_labels_read_cleanly() {
    assert_eq!(normalize_id("Enchanted Book"), "minecraft:enchanted_book");
    assert_eq!(normalize_id("minecraft:COD"), "minecraft:cod");
    assert_eq!(label_of("minecraft:nautilus_shell"), "Nautilus Shell");
    assert!(is_rod("fishing_rod"));
    assert!(is_fish("minecraft:salmon"));
    assert!(is_treasure("name_tag"));
    assert_eq!(sort_catch("minecraft:cod"), CatchKind::Fish);
    assert_eq!(sort_catch("minecraft:saddle"), CatchKind::Treasure);
    assert_eq!(sort_catch("minecraft:stick"), CatchKind::Junk);
}

#[test]
fn protection_names_are_exact_and_independent() {
    assert!(charm_matches("Protection", "minecraft:protection"));
    assert!(!charm_matches("Protection", "minecraft:fire_protection"));
    assert!(!charm_matches("Protection", "minecraft:blast_protection"));
    assert!(!charm_matches("Fire Protection", "minecraft:projectile_protection"));
    assert_eq!(charm_max("Sharpness"), 5);
    assert_eq!(charm_max("Wind Burst"), 3);
}

#[test]
fn worn_rod_trips_at_two_points_left() {
    assert!(rod_is_worn(62, 64));
    assert!(rod_is_worn(63, 64));
    assert!(!rod_is_worn(61, 64));
    assert!(!rod_is_worn(0, 0));
}

#[test]
fn jitter_stays_in_range_and_spreads() {
    let mut seed = 7;
    let mut pre = HashSet::new();
    let mut back = HashSet::new();
    let mut recast = HashSet::new();
    let mut same = 0;
    let mut prev = None;
    for _ in 0..400 {
        let d = jitter_wait(&mut seed, MEAL_PRE_MIN, MEAL_PRE_MAX);
        assert!((MEAL_PRE_MIN..=MEAL_PRE_MAX).contains(&d));
        if prev == Some(d) {
            same += 1;
        }
        prev = Some(d);
        pre.insert(d);
        back.insert(jitter_wait(&mut seed, MEAL_SWAP_BACK_MIN, MEAL_SWAP_BACK_MAX));
        recast.insert(jitter_wait(&mut seed, MEAL_RECAST_MIN, MEAL_RECAST_MAX));
    }
    assert!(pre.len() >= 5);
    assert!(same < 200);
    assert!(back.iter().all(|n| (MEAL_SWAP_BACK_MIN..=MEAL_SWAP_BACK_MAX).contains(n)));
    assert!(recast.iter().all(|n| (MEAL_RECAST_MIN..=MEAL_RECAST_MAX).contains(n)));
    assert!(back.len() >= 8);
    assert!(recast.len() >= 10);
    let mut mid = HashSet::new();
    let mut mid_seed = 3;
    for _ in 0..300 {
        mid.insert(jitter_wait(&mut mid_seed, MEAL_BETWEEN_MIN, MEAL_BETWEEN_MAX));
    }
    assert!(mid.iter().all(|n| (MEAL_BETWEEN_MIN..=MEAL_BETWEEN_MAX).contains(n)));
    assert!(mid.len() >= 8, "{mid:?}");
    assert_eq!((MEAL_PRE_MIN, MEAL_PRE_MAX), (15, 30));
    assert_eq!((MEAL_SWAP_BACK_MIN, MEAL_SWAP_BACK_MAX), (30, 70));
    assert_eq!((MEAL_RECAST_MIN, MEAL_RECAST_MAX), (40, 100));
    assert_eq!((MEAL_BETWEEN_MIN, MEAL_BETWEEN_MAX), (15, 35));
}


#[test]
fn pace_variance_stretches_waits() {
    let mut seed = 99;
    let mut off = HashSet::new();
    let mut on = HashSet::new();
    for _ in 0..300 {
        off.insert(stretched_wait(&mut seed, MEAL_RECAST_MIN, MEAL_RECAST_MAX, false));
        on.insert(stretched_wait(&mut seed, MEAL_RECAST_MIN, MEAL_RECAST_MAX, true));
    }
    assert!(off.iter().all(|n| (MEAL_RECAST_MIN..=MEAL_RECAST_MAX).contains(n)));
    assert!(on.iter().any(|n| *n > MEAL_RECAST_MAX));
    assert!(*on.iter().max().unwrap() > *off.iter().max().unwrap());
    let mut recasts = HashSet::new();
    for _ in 0..80 {
        recasts.insert(recast_pause(&mut seed, false));
    }
    assert!(recasts.iter().all(|n| (15..=24).contains(n)));
    assert!(recasts.len() >= 5);
}

#[test]
fn use_press_needs_two_ticks_after_release_slot_or_throw() {
    assert!(!may_press_use(0, 100, 100));
    assert!(!may_press_use(1, 100, 100));
    assert!(may_press_use(2, 100, 100));
    assert!(!may_press_use(100, 0, 100));
    assert!(!may_press_use(100, 1, 100));
    assert!(may_press_use(100, 2, 100));
    assert!(!may_press_use(100, 100, 0));
    assert!(!may_press_use(100, 100, 1));
    assert!(may_press_use(100, 100, 2));
    assert_eq!(ticks_until_use(0, 100, 100), 2);
    assert_eq!(ticks_until_use(5, 5, 5), 0);
}

#[test]
fn bite_requires_our_bobber_and_respects_settle() {
    assert!(!splash_is_ours(false, Some(0.0)));
    assert!(splash_is_ours(true, Some(3.99)));
    assert!(!splash_is_ours(true, Some(4.0)));
    assert!(!splash_is_ours(true, None));
    assert!(splash_sound("minecraft:entity.fishing_bobber.splash"));
    assert!(splash_sound("entity.fishing_bobber.splash"));
    assert!(!splash_sound("minecraft:entity.fishing_bobber.throw"));
    let mut seed = 1;
    let (cue, wait) = bite_from_splash(true, false, 0, false, true, Some(1.0), true, &mut seed);
    assert_eq!(cue, BiteCue::Wait);
    assert!((BITE_REEL_MIN..=BITE_REEL_MAX).contains(&wait));
    let (cue, _) = bite_from_splash(true, false, 0, false, true, Some(1.0), false, &mut seed);
    assert_eq!(cue, BiteCue::ReelNow);
    let (cue, _) = bite_from_splash(true, true, 0, false, true, Some(1.0), false, &mut seed);
    assert_eq!(cue, BiteCue::Ignore);
    let (cue, _) = bite_from_splash(true, false, 8, false, true, Some(1.0), false, &mut seed);
    assert_eq!(cue, BiteCue::Ignore);
}

#[test]
fn default_toss_keeps_fish_and_throws_usual_junk() {
    let toss = default_toss_set();
    let floors = default_charm_floors();
    assert!(!should_toss("minecraft:salmon", &toss, &[], &floors, false));
    assert!(should_toss("minecraft:lily_pad", &toss, &[], &floors, false));
    assert!(should_toss("minecraft:stick", &toss, &[], &floors, false));
    assert!(!should_toss("minecraft:name_tag", &toss, &[], &floors, false));
    assert!(!should_toss(
        "minecraft:fishing_rod",
        &toss,
        &[],
        &floors,
        true
    ));
}

#[test]
fn item_row_toss_throws_every_book_even_if_charms_look_good() {
    let mut toss = HashSet::new();
    toss.insert("minecraft:enchanted_book".into());
    let floors = default_charm_floors();
    let charms = vec![("minecraft:mending".into(), 1), ("minecraft:unbreaking".into(), 3)];
    assert!(should_toss("minecraft:enchanted_book", &toss, &charms, &floors, false));
}

#[test]
fn unlisted_charms_do_not_vote() {
    let toss = HashSet::new();
    let floors = default_charm_floors();
    let charms = vec![("minecraft:binding_curse".into(), 1)];
    assert!(!should_toss("minecraft:enchanted_book", &toss, &charms, &floors, false));
    assert!(ballots_for(&charms, &floors).is_empty());
}

#[test]
fn below_minimum_is_a_toss_vote() {
    let floors = default_charm_floors();
    let v = charm_ballot("unbreaking", 2, *floors.get("minecraft:unbreaking").unwrap());
    assert!(!v.keep);
    let v = charm_ballot("unbreaking", 3, 3);
    assert!(v.keep);
    let v = charm_ballot("looting", 1, CHARM_ANY);
    assert!(v.keep);
}

#[test]
fn mending_keep_outranks_a_toss_vote() {
    let floors = default_charm_floors();
    let charms = vec![
        ("minecraft:mending".into(), 1),
        ("minecraft:unbreaking".into(), 1),
    ];
    assert!(!should_toss("minecraft:enchanted_book", &HashSet::new(), &charms, &floors, false));
}

#[test]
fn mending_toss_outranks_any_keep_vote() {
    let mut floors = default_charm_floors();
    floors.insert("minecraft:mending".into(), CHARM_TOSS);
    let charms = vec![
        ("minecraft:mending".into(), 1),
        ("minecraft:looting".into(), 3),
    ];
    assert!(should_toss("minecraft:enchanted_book", &HashSet::new(), &charms, &floors, false));
}

#[test]
fn closer_to_max_wins_and_ties_keep() {
    let mut floors = HashMap::new();
    floors.insert("minecraft:looting".into(), 3);
    floors.insert("minecraft:sharpness".into(), CHARM_TOSS);
    // Looting III is at cap (keep) vs Sharpness IV of V (toss) — cap keep wins.
    let keep_looting = vec![
        ("minecraft:looting".into(), 3),
        ("minecraft:sharpness".into(), 4),
    ];
    assert!(!should_toss(
        "minecraft:enchanted_book",
        &HashSet::new(),
        &keep_looting,
        &floors,
        false
    ));
    // Sharpness V (toss, at cap) vs Looting II (keep, not at cap) — cap toss wins.
    floors.insert("minecraft:looting".into(), 1);
    floors.insert("minecraft:sharpness".into(), CHARM_TOSS);
    let toss_sharp = vec![
        ("minecraft:looting".into(), 2),
        ("minecraft:sharpness".into(), 5),
    ];
    assert!(should_toss(
        "minecraft:enchanted_book",
        &HashSet::new(),
        &toss_sharp,
        &floors,
        false
    ));
    // Equal closeness, opposite votes: keep.
    let mut floors = HashMap::new();
    floors.insert("minecraft:looting".into(), 3);
    floors.insert("minecraft:fortune".into(), CHARM_TOSS);
    let tied = vec![
        ("minecraft:looting".into(), 3),
        ("minecraft:fortune".into(), 3),
    ];
    assert!(!should_toss(
        "minecraft:enchanted_book",
        &HashSet::new(),
        &tied,
        &floors,
        false
    ));
}

#[test]
fn stow_only_kept_catch_and_never_a_rod() {
    let toss = default_toss_set();
    let floors = default_charm_floors();
    assert!(should_stow("minecraft:salmon", &toss, &[], &floors));
    assert!(!should_stow("minecraft:lily_pad", &toss, &[], &floors));
    assert!(!should_stow("minecraft:fishing_rod", &toss, &[], &floors));
    assert!(!should_stow("minecraft:diamond", &toss, &[], &floors));
    assert!(policy::may_begin_stow(false, 0));
    assert!(!policy::may_begin_stow(true, 0));
    assert!(!policy::may_begin_stow(false, 8));
    assert!(policy::stow_may_click(true, true, true));
    assert!(!policy::stow_may_click(true, false, true));
}

#[test]
fn hunger_trips_until_the_bar_is_full() {
    assert!(hunger_needs_food(14, 20.0, 20.0));
    assert!(hunger_needs_food(15, 20.0, 20.0));
    assert!(hunger_needs_food(19, 20.0, 20.0));
    assert!(!hunger_needs_food(20, 18.0, 20.0));
    assert!(!hunger_needs_food(20, 20.0, 20.0));
}

#[test]
fn food_origin_prefers_offhand_then_hotbar_and_ignores_the_bag() {
    let mut hotbar: [Option<String>; 9] = Default::default();
    hotbar[0] = Some("minecraft:cooked_cod".into());
    hotbar[8] = Some("minecraft:bread".into());
    let src = pick_food_origin(Some("minecraft:cooked_salmon"), &hotbar, 4);
    assert_eq!(src, Some(FoodOrigin::Offhand));
    let src = pick_food_origin(None, &hotbar, 8);
    assert_eq!(src, Some(FoodOrigin::Hotbar { slot: 8 }));
    assert!(!src.as_ref().unwrap().needs_swap(8));
    hotbar[8] = None;
    let src = pick_food_origin(None, &hotbar, 4);
    assert_eq!(src, Some(FoodOrigin::Hotbar { slot: 0 }));
    assert!(src.as_ref().unwrap().needs_swap(4));
    hotbar[0] = None;
    let src = pick_food_origin(None, &hotbar, 4);
    assert_eq!(src, None, "main-inventory food is not a meal source");
}

#[test]
fn risky_food_is_skipped_while_safer_hotbar_food_exists() {
    let mut hotbar: [Option<String>; 9] = Default::default();
    hotbar[1] = Some("minecraft:rotten_flesh".into());
    let src = pick_food_origin(None, &hotbar, 0);
    assert_eq!(src, Some(FoodOrigin::Hotbar { slot: 1 }));
    hotbar[3] = Some("minecraft:bread".into());
    let src = pick_food_origin(None, &hotbar, 0);
    assert_eq!(src, Some(FoodOrigin::Hotbar { slot: 3 }));
    let src = pick_food_origin(Some("minecraft:spider_eye"), &hotbar, 0);
    assert_eq!(src, Some(FoodOrigin::Hotbar { slot: 3 }));
}

fn trace_meal(
    hook_out: bool,
    need_swap: bool,
    pre: i32,
    after_swap: i32,
    between: i32,
    back: i32,
    recast: i32,
    bites: i32,
) -> Vec<(MealPhase, MealStep, i32)> {
    let mut phase = MealPhase::Rest;
    let mut timer = 0;
    let mut since_release = 100;
    let mut since_slot = 100;
    let mut still = true;
    let mut remaining = bites;
    let mut steps = Vec::new();
    for _ in 0..400 {
        let can_use = may_press_use(since_release, since_slot, 100);
        let hook = hook_out && phase == MealPhase::Rest;
        let swap = need_swap
            && matches!(
                phase,
                MealPhase::Rest | MealPhase::AfterReel | MealPhase::BetweenBites
            );
        let about_to_finish = phase == MealPhase::Chewing && timer <= 1;
        let hungry = if about_to_finish { remaining > 1 } else { remaining > 0 };
        let adv = step_meal(
            phase,
            timer,
            true,
            hungry,
            remaining > 0,
            hook,
            false,
            still,
            can_use,
            swap,
            pre,
            after_swap,
            between,
            back,
            recast,
        );
        steps.push((adv.phase, adv.step, adv.timer));
        match adv.step {
            MealStep::SwapFood | MealStep::SwapRod => {
                since_slot = 0;
                since_release = since_release.saturating_add(1);
                still = adv.step == MealStep::SwapFood;
            }
            MealStep::LetGo => {
                since_release = 0;
                since_slot = since_slot.saturating_add(1);
                remaining = (remaining - 1).max(0);
            }
            _ => {
                since_release = since_release.saturating_add(1);
                since_slot = since_slot.saturating_add(1);
            }
        }
        phase = adv.phase;
        timer = adv.timer;
        if adv.step == MealStep::CastAgain && phase == MealPhase::Rest {
            break;
        }
    }
    steps
}

#[test]
fn meal_splits_reel_swap_chew_and_post_meal_pauses() {
    let steps = trace_meal(true, true, 15, 3, 20, 30, 40, 1);
    let kinds: Vec<MealStep> = steps.iter().map(|s| s.1).collect();
    assert_eq!(kinds[0], MealStep::Reel);
    assert_eq!(steps[0].0, MealPhase::AfterReel);
    assert_eq!(steps[0].2, 15);
    let swap_i = kinds.iter().position(|s| *s == MealStep::SwapFood).unwrap();
    let chew_i = kinds.iter().position(|s| *s == MealStep::StartChew).unwrap();
    let letgo = kinds.iter().position(|s| *s == MealStep::LetGo).unwrap();
    let rod = kinds.iter().position(|s| *s == MealStep::SwapRod).unwrap();
    let recast = kinds.iter().position(|s| *s == MealStep::CastAgain).unwrap();
    assert!(swap_i < chew_i && chew_i < letgo && letgo < rod && rod < recast);
    assert_eq!(steps[letgo].0, MealPhase::BeforeRod);
    assert_eq!(steps[letgo].2, 30);
    assert_eq!(steps[rod].2, 40);
    assert!(recast - letgo >= 2);
}

#[test]
fn meal_pauses_between_bites_until_full() {
    let steps = trace_meal(false, false, 15, 3, 20, 30, 40, 2);
    let kinds: Vec<MealStep> = steps.iter().map(|s| s.1).collect();
    let chews: Vec<_> = kinds
        .iter()
        .enumerate()
        .filter(|(_, s)| **s == MealStep::StartChew)
        .map(|(i, _)| i)
        .collect();
    let letgos: Vec<_> = kinds
        .iter()
        .enumerate()
        .filter(|(_, s)| **s == MealStep::LetGo)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(chews.len(), 2, "two bites: {kinds:?}");
    assert_eq!(letgos.len(), 2);
    assert_eq!(steps[letgos[0]].0, MealPhase::BetweenBites);
    assert_eq!(steps[letgos[0]].2, 20);
    assert!(chews[1] > letgos[0] + 1, "second bite must wait after the first release");
    assert_eq!(steps[letgos[1]].0, MealPhase::BeforeRod);
    let between = &kinds[letgos[0]..=chews[1]];
    assert!(between.iter().all(|s| *s != MealStep::SwapRod));
    assert!(between.iter().all(|s| *s != MealStep::CastAgain));
    assert!(chews[1] - chews[0] > 2);
}

#[test]
fn meal_never_uses_on_the_same_step_as_a_swap_or_release() {
    for hook in [true, false] {
        for need in [true, false] {
            let steps = trace_meal(hook, need, 15, 2, 20, 30, 40, 2);
            let kinds: Vec<MealStep> = steps.iter().map(|s| s.1).collect();
            for (i, step) in kinds.iter().enumerate() {
                if matches!(*step, MealStep::StartChew | MealStep::CastAgain | MealStep::Reel)
                    && i > 0
                {
                    assert!(!matches!(
                        kinds[i - 1],
                        MealStep::SwapFood | MealStep::SwapRod | MealStep::LetGo
                    ));
                }
            }
        }
    }
}

#[test]
fn meal_holds_cast_until_use_gap_after_release_and_rod_swap() {
    let steps = trace_meal(false, false, 15, 2, 20, 1, 1, 1);
    let kinds: Vec<MealStep> = steps.iter().map(|s| s.1).collect();
    let letgo = kinds.iter().position(|s| *s == MealStep::LetGo).unwrap();
    let rod = kinds.iter().position(|s| *s == MealStep::SwapRod).unwrap();
    let recast = kinds.iter().position(|s| *s == MealStep::CastAgain).unwrap();
    assert!(recast - letgo >= 2);
    assert!(recast - rod >= 2);
}

#[test]
fn meal_does_not_start_during_a_manual_use() {
    let adv = step_meal(
        MealPhase::Rest,
        0,
        true,
        true,
        true,
        false,
        true,
        true,
        true,
        false,
        15,
        2,
        20,
        30,
        40,
    );
    assert_eq!(adv.step, MealStep::Idle);
    assert_eq!(adv.phase, MealPhase::Rest);
}

#[test]
fn config_round_trip_keeps_player_choices() {
    let mut p = Prefs::default();
    assert!(p.pace && p.bail_hurt && p.rest_after_lull);
    assert!(!p.meals && !p.running);
    p.meals = true;
    p.toss.remove("minecraft:lily_pad");
    p.chime.insert("minecraft:salmon".into());
    p.charm_floor.insert("minecraft:looting".into(), 3);
    let text = dump_prefs(&p);
    assert!(text.contains("format=1"));
    assert!(text.contains("meals=yes"));
    // Config keys must be Hookline's, not leftovers from other helpers.
    for stale in ["human".to_string() + "_delay", "auto".to_string() + "_eat", "junk".to_string() + "_items"] {
        assert!(!text.contains(&stale), "{stale}");
    }
    let (loaded, keys) = load_prefs(&text);
    assert!(loaded.meals);
    assert!(!loaded.toss.contains("minecraft:lily_pad"));
    assert!(loaded.chime.contains("minecraft:salmon"));
    assert_eq!(loaded.charm_floor.get("minecraft:looting"), Some(&3));
    assert!(keys.contains("meals"));
    let mut native = HashMap::new();
    native.insert("meals".into(), "no".into());
    let mut overlaid = loaded.clone();
    persist::apply_native_overlay(&mut overlaid, &native, &keys);
    assert!(overlaid.meals, "a leftover native off must not wipe a saved yes");
}

#[test]
fn missing_toss_key_keeps_defaults_empty_value_tosses_nothing() {
    let (p, _) = load_prefs("format=1\nmeals=yes\n");
    assert!(p.toss.contains("minecraft:bowl"));
    let (p, _) = load_prefs("format=1\ntoss=\n");
    assert!(p.toss.is_empty());
}

#[test]
fn ledger_rates_use_hooks_and_treasure_over_casts() {
    let mut t = SessionTally::default();
    assert_eq!(t.hook_rate(), "0%");
    t.casts = 4;
    t.hooks = 2;
    t.treasure = 1;
    assert_eq!(t.hook_rate(), "50%");
    assert_eq!(t.treasure_rate(), "25%");
}

#[test]
fn charm_chip_cycles_toss_any_then_levels() {
    assert_eq!(persist::cycle_charm_floor(CHARM_TOSS, 3), CHARM_ANY);
    assert_eq!(persist::cycle_charm_floor(CHARM_ANY, 3), 1);
    assert_eq!(persist::cycle_charm_floor(1, 3), 2);
    assert_eq!(persist::cycle_charm_floor(3, 3), CHARM_TOSS);
}

#[test]
fn lull_window_is_five_minutes() {
    assert_eq!(LULL_TICKS, 6000);
}


fn mark_shown(total: usize, view_h: f32) -> Vec<bool> {
    use hookline_logic::listing::{clamp_scroll, max_scroll, shown_rows};
    let mut seen = vec![false; total];
    let max = max_scroll(total, view_h);
    let last = max.ceil() as i32;
    for raw in 0..=last {
        let scroll = clamp_scroll(raw as f32, total, view_h);
        let (start, end) = shown_rows(scroll, total, view_h);
        for i in start..end {
            seen[i] = true;
        }
    }
    let (start, end) = shown_rows(max, total, view_h);
    for i in start..end {
        seen[i] = true;
    }
    seen
}

fn assert_every_row_reachable(total: usize, view_h: f32) {
    let seen = mark_shown(total, view_h);
    let missing: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter(|(_, on)| !**on)
        .map(|(i, _)| i)
        .collect();
    assert!(
        missing.is_empty(),
        "view_h={view_h} total={total} missing={missing:?}"
    );
}

#[test]
fn loot_list_reaches_every_row_at_every_panel_height() {
    use hookline_logic::listing::{body_h, loot_view_h, PANEL_H_MAX, PANEL_H_MIN};
    assert_eq!(CATCH_ROWS.last().copied(), Some("minecraft:leather"));
    for panel in [PANEL_H_MIN, 240.0, 280.0, PANEL_H_MAX] {
        assert_every_row_reachable(CATCH_ROWS.len(), loot_view_h(body_h(panel)));
    }
}

#[test]
fn enchant_list_reaches_every_row_at_every_panel_height() {
    use hookline_logic::listing::{body_h, charm_view_h, PANEL_H_MAX, PANEL_H_MIN};
    for panel in [PANEL_H_MIN, 240.0, 280.0, PANEL_H_MAX] {
        assert_every_row_reachable(CHARMS.len(), charm_view_h(body_h(panel)));
    }
}

#[test]
fn scrollbar_thumb_tracks_offset_and_hides_when_the_list_fits() {
    use hookline_logic::listing::{max_scroll, thumb, THUMB_MIN};
    let view = 120.0;
    let track = 120.0;
    assert!(thumb(0.0, 3, view, track).is_none());
    let total = CATCH_ROWS.len();
    let top = thumb(0.0, total, view, track).expect("overflowing loot list");
    let max = max_scroll(total, view);
    let bot = thumb(max, total, view, track).expect("thumb at end");
    assert!((top.y - 0.0).abs() < 0.01);
    assert!((bot.y + bot.h - track).abs() < 0.05, "{bot:?}");
    assert!(top.h >= THUMB_MIN);
    assert!((top.h - bot.h).abs() < 0.01);
}
