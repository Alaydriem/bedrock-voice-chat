use common::structs::i18n::{LanguagePack, PackEntry};
use std::collections::HashMap;

fn russian() -> LanguagePack {
    let mut m = HashMap::new();
    m.insert(
        "Sign In Again".to_string(),
        PackEntry::One("Войти снова".into()),
    );
    m.insert("audio\u{4}Output".to_string(), PackEntry::One("Выход".into()));
    m.insert(
        "{count} player nearby".to_string(),
        PackEntry::Many(vec!["игрок".into(), "игрока".into(), "игроков".into()]),
    );
    LanguagePack {
        v: 1,
        locale: "ru".into(),
        plural: vec!["one".into(), "few".into(), "many".into()],
        m,
    }
}

#[test]
fn a_missing_msgid_is_absent_rather_than_empty() {
    assert_eq!(russian().lookup("Never Translated"), None);
}

#[test]
fn context_keys_are_built_with_the_eot_separator_both_runtimes_use() {
    assert_eq!(russian().lookup_context("audio", "Output"), Some("Выход"));
}

#[test]
fn a_context_free_lookup_does_not_find_a_context_scoped_entry() {
    assert_eq!(russian().lookup("Output"), None);
}

#[test]
fn plural_category_maps_to_its_index_in_the_declared_order() {
    let pack = russian();
    assert_eq!(pack.plural_index("one"), 0);
    assert_eq!(pack.plural_index("few"), 1);
    assert_eq!(pack.plural_index("many"), 2);
}

#[test]
fn a_category_the_locale_never_uses_for_integers_falls_to_the_last_form() {
    let pack = russian();
    assert_eq!(pack.plural_index("other"), 2);
}

#[test]
fn plural_lookup_selects_the_form_for_the_category() {
    let pack = russian();
    assert_eq!(
        pack.lookup_plural("{count} player nearby", "few"),
        Some("игрока")
    );
}

#[test]
fn plural_lookup_on_a_singular_entry_returns_that_single_form() {
    let mut m = HashMap::new();
    m.insert("Nearby".to_string(), PackEntry::One("Рядом".into()));
    let pack = LanguagePack {
        v: 1,
        locale: "ru".into(),
        plural: vec!["other".into()],
        m,
    };
    assert_eq!(pack.lookup_plural("Nearby", "one"), Some("Рядом"));
}
