use bvc_client_lib::i18n::Localizer;
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
fn with_no_pack_a_message_id_is_its_own_english_translation() {
    let localizer = Localizer::new(None);

    assert_eq!(localizer.t("Sign In Again"), "Sign In Again");
}

#[test]
fn a_loaded_pack_translates_a_known_message() {
    let localizer = Localizer::new(Some(russian()));

    assert_eq!(localizer.t("Sign In Again"), "Войти снова");
}

#[test]
fn an_untranslated_message_falls_through_to_english() {
    let localizer = Localizer::new(Some(russian()));

    assert_eq!(localizer.t("Never Translated"), "Never Translated");
}

#[test]
fn context_lookups_use_the_separator_the_compiler_emits() {
    let localizer = Localizer::new(Some(russian()));

    assert_eq!(localizer.tc("audio", "Output"), "Выход");
}

// The categories ICU names for these counts are the same ones the pack compiler wrote into
// `plural`. If either side ever renamed them, this is where the two would stop agreeing.
#[test]
fn icu_plural_categories_index_the_forms_the_compiler_ordered() {
    let localizer = Localizer::new(Some(russian()));
    let cases = [(1, "игрок"), (3, "игрока"), (7, "игроков"), (21, "игрок")];

    for (count, expected) in cases {
        assert_eq!(
            localizer.tn("{count} player nearby", "{count} players nearby", count),
            expected,
            "count {count}"
        );
    }
}

#[test]
fn with_no_pack_plurals_use_the_two_english_forms() {
    let localizer = Localizer::new(None);

    assert_eq!(
        localizer.tn("{count} player nearby", "{count} players nearby", 1),
        "{count} player nearby"
    );
    assert_eq!(
        localizer.tn("{count} player nearby", "{count} players nearby", 4),
        "{count} players nearby"
    );
}

#[test]
fn an_unparseable_locale_leaves_the_pack_readable_for_singulars() {
    let mut pack = russian();
    pack.locale = "!! not a locale !!".into();
    let localizer = Localizer::new(Some(pack));

    assert_eq!(localizer.t("Sign In Again"), "Войти снова");
}
