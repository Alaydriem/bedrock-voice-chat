use common::structs::reachability::AddressFamilyPreference;

#[derive(Debug, Default, Clone)]
pub struct DeviceSnapshot {
    pub input_name: Option<String>,
    pub input_sample_rate: Option<u32>,
    pub output_name: Option<String>,
    pub output_sample_rate: Option<u32>,
    pub muted_peer_count: u32,
    // Read on the same slow refresh as the devices, because it lives behind the same lock.
    pub family_preference: Option<AddressFamilyPreference>,
}
