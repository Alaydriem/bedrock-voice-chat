use common::GameDataCollection;
use common::game_data::Dimension;
use common::traits::player_data::PlayerData;

fn player(name: &str, dimension: &str) -> String {
    format!(
        r#"{{"name":"{name}",{dimension}"coordinates":{{"x":0,"y":64,"z":0}},"deafen":false,"orientation":{{"x":0,"y":0}}}}"#
    )
}

#[test]
fn a_bad_record_drops_only_that_player() {
    let batch = format!(
        r#"{{"game":"minecraft","players":[{},{},{}]}}"#,
        player("Valid", r#""dimension":"overworld","#),
        player("NullDimension", r#""dimension":null,"#),
        player("MissingDimension", ""),
    );

    let collection: GameDataCollection =
        serde_json::from_str(&batch).expect("a batch with one bad record still parses");

    let names: Vec<&str> = collection.players.iter().map(|p| p.get_name()).collect();
    assert_eq!(names, vec!["Valid"]);
}

#[test]
fn a_custom_dimension_record_is_accepted() {
    let batch = format!(
        r#"{{"game":"minecraft","players":[{}]}}"#,
        player("Explorer", r#""dimension":"twilightforest:twilight_forest","#),
    );

    let collection: GameDataCollection = serde_json::from_str(&batch).unwrap();

    assert_eq!(collection.players.len(), 1);
    assert_eq!(
        collection.players[0].dimension(),
        Some(Dimension::parse("twilightforest:twilight_forest"))
    );
}
