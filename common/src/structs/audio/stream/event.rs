use ts_rs::TS;

#[derive(Debug, Clone, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum StreamEvent {
    Mute,
    Record,
}
