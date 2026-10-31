/// One row of the gamertag gate: a gamertag, what a beta.27 mod and the current mod
/// report for that player, and the platform UUID they arrive with.
///
/// Each row is one way Floodgate can turn a gamertag into a Java name: spaces, truncation,
/// prefixes, non-Latin text, linked accounts. Linked players carry a Java account UUID;
/// everyone else Floodgate's `UUID(0, xuid)`.
#[derive(Debug, Clone, Copy)]
pub struct GateRow {
    pub id: &'static str,
    pub gamertag: &'static str,
    pub beta27: (&'static str, Option<&'static str>),
    pub new: (&'static str, Option<&'static str>),
    pub uuid: &'static str,
}

pub struct GateRows;

impl GateRows {
    pub const FLOODGATE_UUID: &'static str = "00000000-0000-0000-0009-01f2a3b4c5d6";
    pub const FLOODGATE_UUID_2: &'static str = "00000000-0000-0000-0009-01f2a3b4c5d7";
    pub const JAVA_UUID: &'static str = "3c9a1b77-2d44-4e18-8b6c-000000000002";
    pub const JAVA_UUID_2: &'static str = "3c9a1b77-2d44-4e18-8b6c-000000000003";

    pub const ALL: &'static [GateRow] = &[
        GateRow { id: "G01", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G02", gamertag: "TTV 3RICtion", beta27: (".TTV_3RICtion", Some("TTV 3RICtion")), new: ("TTV 3RICtion", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G03", gamertag: "A B C D", beta27: (".A_B_C_D", Some("A B C D")), new: ("A B C D", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G04", gamertag: "Lord Of Pixels1", beta27: (".Lord_Of_Pixels1", Some("Lord Of Pixels1")), new: ("Lord Of Pixels1", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G05", gamertag: "LongGamertag123", beta27: ("LongGamertag123", None), new: ("LongGamertag123", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G06", gamertag: "LongGamertag123", beta27: ("BE_LongGamertag1", Some("LongGamertag123")), new: ("LongGamertag123", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G07", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G08", gamertag: "TTV 3RICtion", beta27: ("TTV 3RICtion", None), new: ("TTV 3RICtion", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G09", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G10", gamertag: "TTV 3RICtion", beta27: ("TTV_3RICtion", Some("TTV 3RICtion")), new: ("TTV 3RICtion", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G11", gamertag: "テスト ユーザー", beta27: (".テスト_ユーザー", Some("テスト ユーザー")), new: ("テスト ユーザー", None), uuid: Self::FLOODGATE_UUID },
        GateRow { id: "G12", gamertag: "TTV 3RICtion", beta27: ("Eric_J", Some("TTV 3RICtion")), new: ("Eric_J", Some("TTV 3RICtion")), uuid: Self::JAVA_UUID },
        GateRow { id: "G13", gamertag: "Steve", beta27: ("Steve", None), new: ("Steve", None), uuid: Self::JAVA_UUID },
        GateRow { id: "G14", gamertag: "LongGamertag123", beta27: ("Notch", Some("LongGamertag123")), new: ("Notch", Some("LongGamertag123")), uuid: Self::JAVA_UUID },
        GateRow { id: "G17", gamertag: "Notch", beta27: ("Notch", None), new: ("Notch", None), uuid: Self::JAVA_UUID },
    ];
}
