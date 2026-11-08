use std::sync::Arc;

use common::structs::app::DeviceInfo;
use sentry::protocol::{Context, DeviceContext, Event, Log, LogAttribute, OsContext};

use crate::analytics::PlatformId;

/// Stamps the platform ID and the device description onto everything sent to Sentry.
///
/// Applied in `before_send` and `before_send_log` rather than on a scope. A Sentry scope is
/// copied into each thread's hub the first time that thread reports, so a scope change made later
/// (a platform ID refreshed from the About pane, on a tokio thread) never reaches threads that
/// had already reported. The send callbacks run for every event and log from every thread.
pub struct SentryEnricher {
    platform_id: Arc<PlatformId>,
    device: Option<DeviceInfo>,
}

impl SentryEnricher {
    pub fn new(platform_id: Arc<PlatformId>, device: Option<DeviceInfo>) -> Self {
        Self {
            platform_id,
            device,
        }
    }

    pub fn new_shared(platform_id: Arc<PlatformId>, device: Option<DeviceInfo>) -> Arc<Self> {
        Arc::new(Self::new(platform_id, device))
    }

    pub fn device(&self) -> Option<DeviceInfo> {
        self.device.clone()
    }

    // Empty until setup has read the stored ID. Reporting no user beats reporting an empty one.
    fn user_id(&self) -> Option<String> {
        let id = self.platform_id.get();
        (!id.is_empty()).then_some(id)
    }

    pub fn enrich_event(&self, mut event: Event<'static>) -> Event<'static> {
        if let Some(id) = self.user_id() {
            let mut user = event.user.take().unwrap_or_default();
            user.id = Some(id);
            event.user = Some(user);
        }

        let Some(device) = self.device.as_ref() else {
            return event;
        };

        // Merged into what the context integration already wrote, so its `arch` and kernel
        // version survive where the probe has nothing better.
        let mut os = match event.contexts.remove("os") {
            Some(Context::Os(os)) => *os,
            _ => OsContext::default(),
        };
        os.name = Some(device.os_name.clone());
        os.version = device.os_version.clone();
        os.build = device.os_build.clone();
        if device.kernel_version.is_some() {
            os.kernel_version = device.kernel_version.clone();
        }
        event.contexts.insert("os".to_string(), os.into());

        let mut hardware = match event.contexts.remove("device") {
            Some(Context::Device(hardware)) => *hardware,
            _ => DeviceContext::default(),
        };
        hardware.model = device.model.clone();
        hardware.model_id = device.model_id.clone();
        hardware.family = device.family.clone();
        if let Some(manufacturer) = device.manufacturer.as_ref() {
            hardware
                .other
                .insert("manufacturer".to_string(), manufacturer.clone().into());
        }
        if let Some(brand) = device.brand.as_ref() {
            hardware
                .other
                .insert("brand".to_string(), brand.clone().into());
        }
        event.contexts.insert("device".to_string(), hardware.into());

        event
    }

    pub fn enrich_log(&self, mut log: Log) -> Log {
        if let Some(id) = self.user_id() {
            log.attributes
                .insert("user.id".to_string(), LogAttribute(id.into()));
        }

        let Some(device) = self.device.as_ref() else {
            return log;
        };

        // Overrides the `os.name` / `os.version` the SDK cached at startup from `uname`, which
        // on a phone reads `Linux` or `Darwin` with a kernel version.
        log.attributes.insert(
            "os.name".to_string(),
            LogAttribute(device.os_name.clone().into()),
        );
        let optional = [
            ("os.version", &device.os_version),
            ("os.build", &device.os_build),
            ("os.kernel_version", &device.kernel_version),
            ("device.manufacturer", &device.manufacturer),
            ("device.brand", &device.brand),
            ("device.model", &device.model),
            ("device.model_id", &device.model_id),
            ("device.family", &device.family),
        ];
        for (key, value) in optional {
            if let Some(value) = value {
                log.attributes
                    .insert(key.to_string(), LogAttribute(value.clone().into()));
            }
        }

        log
    }

    /// The `before_send` / `before_send_log` callbacks for `sentry::ClientOptions`.
    pub fn install(self: &Arc<Self>, options: &mut sentry::ClientOptions) {
        let events = self.clone();
        options.before_send = Some(Arc::new(move |event| Some(events.enrich_event(event))));
        let logs = self.clone();
        options.before_send_log = Some(Arc::new(move |log| Some(logs.enrich_log(log))));
    }
}
