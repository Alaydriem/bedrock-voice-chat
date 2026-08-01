use chrono::{DateTime, Utc};
use uuid::Uuid;

/// First-launch identity for cohort analysis. Resolution is pure so it can be
/// exercised without an `AppHandle` — a struct holding one drags the whole Tauri
/// GUI stack into a test binary through drop glue.
pub struct InstallMarker {
    pub install_id: String,
    pub install_date: String,
    pub is_first_run: bool,
}

impl InstallMarker {
    const DATE_FORMAT: &'static str = "%Y-%m-%d";

    /// `stored_id` and `stored_date` are the values already in `store.json`.
    /// A blank or nil id is treated as absent, matching the existing resolution.
    pub fn resolve(stored_id: Option<&str>, stored_date: Option<&str>, now: DateTime<Utc>) -> Self {
        let existing = stored_id
            .map(str::trim)
            .filter(|id| !id.is_empty() && *id != Uuid::nil().to_string());

        match existing {
            Some(id) => {
                let install_date = Self::date_from_v7(id)
                    .or_else(|| stored_date.map(str::to_string))
                    .unwrap_or_else(|| now.format(Self::DATE_FORMAT).to_string());
                Self {
                    install_id: id.to_string(),
                    install_date,
                    is_first_run: false,
                }
            }
            None => Self {
                install_id: Uuid::now_v7().to_string(),
                install_date: now.format(Self::DATE_FORMAT).to_string(),
                is_first_run: true,
            },
        }
    }

    /// A version-7 UUID carries its creation time in its high bits, so the install
    /// date of an id already in the wild needs no separate record.
    ///
    /// The version check is load-bearing. `get_timestamp` also succeeds for v1 and
    /// v6, which count from the Gregorian epoch — a v1 id would silently decode to
    /// a 1998 install date instead of falling through to the persisted value.
    fn date_from_v7(id: &str) -> Option<String> {
        let uuid = Uuid::parse_str(id).ok()?;
        if uuid.get_version_num() != 7 {
            return None;
        }
        let (secs, nanos) = uuid.get_timestamp()?.to_unix();
        let stamp = DateTime::<Utc>::from_timestamp(secs as i64, nanos)?;
        Some(stamp.format(Self::DATE_FORMAT).to_string())
    }
}
