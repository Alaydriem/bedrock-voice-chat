use bvc_client_lib::i18n::LocalizationService;
use std::path::Path;
use tempfile::TempDir;

fn write(directory: &Path, name: &str, body: &str) {
    std::fs::write(directory.join(name), body).expect("pack should be writable");
}

fn russian() -> &'static str {
    r#"{"v":1,"locale":"ru","plural":["one","few","many"],
        "m":{"Sign In Again":"Войти снова",
             "{count} player nearby":["игрок","игрока","игроков"]}}"#
}

fn packs() -> TempDir {
    let directory = TempDir::new().expect("temp dir should be creatable");
    write(directory.path(), "ru.json", russian());
    write(directory.path(), "de.json", r#"{"v":1,"locale":"de","plural":["one","other"],"m":{}}"#);
    write(directory.path(), "notes.txt", "not a pack");
    directory
}

#[test]
fn only_json_files_are_offered_as_locales() {
    let directory = packs();
    let service = LocalizationService::new(directory.path().to_path_buf());

    assert_eq!(service.available(), vec!["de".to_string(), "ru".to_string()]);
}

#[test]
fn a_missing_directory_offers_nothing_rather_than_failing() {
    let service = LocalizationService::new(Path::new("does/not/exist").to_path_buf());

    assert!(service.available().is_empty());
}

#[test]
fn an_os_locale_with_a_region_resolves_to_the_bare_language_pack() {
    let directory = packs();
    let service = LocalizationService::new(directory.path().to_path_buf());

    assert_eq!(service.resolve("ru-RU"), Some("ru".to_string()));
}

#[test]
fn a_locale_that_is_not_shipped_resolves_to_nothing_so_english_is_used() {
    let directory = packs();
    let service = LocalizationService::new(directory.path().to_path_buf());

    assert_eq!(service.resolve("is-IS"), None);
}

#[test]
fn a_shipped_pack_loads_with_its_messages() {
    let directory = packs();
    let service = LocalizationService::new(directory.path().to_path_buf());

    let pack = service.load("ru").expect("load should succeed").expect("ru is shipped");

    assert_eq!(pack.locale, "ru");
    assert_eq!(pack.lookup("Sign In Again"), Some("Войти снова"));
}

#[test]
fn plural_forms_survive_the_trip_from_the_compiler() {
    let directory = packs();
    let service = LocalizationService::new(directory.path().to_path_buf());

    let pack = service.load("ru").expect("load should succeed").expect("ru is shipped");

    assert_eq!(pack.lookup_plural("{count} player nearby", "few"), Some("игрока"));
}

#[test]
fn a_locale_with_no_pack_is_absent_rather_than_an_error() {
    let directory = packs();
    let service = LocalizationService::new(directory.path().to_path_buf());

    assert!(service.load("ja").expect("load should succeed").is_none());
}

// A pack is content the app did not author. Refusing to start over a malformed one would
// turn a bad translation into a dead application, so it degrades to English instead.
#[test]
fn a_malformed_pack_falls_back_to_english_rather_than_erroring() {
    let directory = TempDir::new().expect("temp dir should be creatable");
    write(directory.path(), "ru.json", "{ this is not json");
    let service = LocalizationService::new(directory.path().to_path_buf());

    assert!(service.load("ru").expect("load should not error").is_none());
}
