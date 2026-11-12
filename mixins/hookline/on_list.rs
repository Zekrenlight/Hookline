fn hookline_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::hookline as hook;
    mods.push(crate::game::native_hooks::NativeMod {
        id: hook::MOD_ID.into(),
        title: format!("{} {}", hook::TITLE, hook::VERSION),
        description: hook::BLURB.into(),
        keys: hook::BINDINGS
            .iter()
            .map(|[n, l, d]| [n.to_string(), l.to_string(), d.to_string()])
            .collect(),
    });
}
