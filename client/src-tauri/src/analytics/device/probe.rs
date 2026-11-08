use common::structs::app::DeviceInfo;

// Vendor properties that carry the retail name where `ro.product.model` holds only a code.
// OnePlus reports `CPH2655` as its model and `OnePlus 13` here. Pixels need none of these: their
// model is already the retail name.
const ANDROID_MARKET_NAME_PROPERTIES: &[&str] = &[
    "ro.product.marketname",
    "ro.product.vendor.marketname",
    "ro.vendor.oplus.market.name",
    "ro.oppo.market.name",
    "ro.config.marketing_name",
    "ro.vendor.product.display",
];

/// Reads the operating system and hardware of a mobile client.
pub struct DeviceProbe;

impl DeviceProbe {
    /// The device this process runs on, or `None` on desktop.
    pub fn current() -> Option<DeviceInfo> {
        #[cfg(target_os = "android")]
        {
            Some(Self::from_android(Self::system_property, Self::kernel_release()))
        }
        #[cfg(target_os = "ios")]
        {
            Some(Self::from_apple(Self::sysctl, Self::kernel_release()))
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            None
        }
    }

    /// Builds the Android description from system properties, read through `property`.
    pub fn from_android(
        property: impl Fn(&str) -> Option<String>,
        kernel_version: Option<String>,
    ) -> DeviceInfo {
        let model_id = property("ro.product.model");
        let market_name = ANDROID_MARKET_NAME_PROPERTIES
            .iter()
            .find_map(|name| property(name));

        DeviceInfo {
            os_name: "Android".to_string(),
            os_version: property("ro.build.version.release"),
            os_build: property("ro.build.version.sdk"),
            kernel_version,
            manufacturer: property("ro.product.manufacturer"),
            brand: property("ro.product.brand"),
            model: market_name.or_else(|| model_id.clone()),
            model_id,
            family: None,
        }
    }

    /// Builds the iOS description from sysctl values, read through `sysctl`.
    pub fn from_apple(
        sysctl: impl Fn(&str) -> Option<String>,
        kernel_version: Option<String>,
    ) -> DeviceInfo {
        // `hw.machine` is the hardware identifier (`iPhone17,3`). `hw.model` is the board, the
        // reverse of macOS.
        let machine = sysctl("hw.machine");
        let family = machine.as_deref().and_then(Self::apple_family);
        let os_name = match family.as_deref() {
            Some("iPad") => "iPadOS",
            _ => "iOS",
        };

        DeviceInfo {
            os_name: os_name.to_string(),
            os_version: sysctl("kern.osproductversion"),
            os_build: sysctl("kern.osversion"),
            kernel_version,
            manufacturer: Some("Apple".to_string()),
            brand: Some("Apple".to_string()),
            model: machine,
            model_id: sysctl("hw.model"),
            family,
        }
    }

    // The alphabetic prefix of the hardware identifier: `iPhone17,3` -> `iPhone`.
    fn apple_family(machine: &str) -> Option<String> {
        let family: String = machine
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        (!family.is_empty()).then_some(family)
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn kernel_release() -> Option<String> {
        let mut info: libc::utsname = unsafe { std::mem::zeroed() };
        if unsafe { libc::uname(&mut info) } != 0 {
            return None;
        }
        let release = unsafe { std::ffi::CStr::from_ptr(info.release.as_ptr()) };
        Self::non_empty(release.to_string_lossy().into_owned())
    }

    #[cfg(target_os = "android")]
    fn system_property(name: &str) -> Option<String> {
        let key = std::ffi::CString::new(name).ok()?;
        let mut value = [0 as libc::c_char; libc::PROP_VALUE_MAX as usize];
        let len = unsafe { libc::__system_property_get(key.as_ptr(), value.as_mut_ptr()) };
        if len <= 0 {
            return None;
        }
        let value = unsafe { std::ffi::CStr::from_ptr(value.as_ptr()) };
        Self::non_empty(value.to_string_lossy().into_owned())
    }

    #[cfg(target_os = "ios")]
    fn sysctl(name: &str) -> Option<String> {
        let key = std::ffi::CString::new(name).ok()?;
        let mut size: libc::size_t = 0;
        let probe = unsafe {
            libc::sysctlbyname(
                key.as_ptr(),
                std::ptr::null_mut(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if probe != 0 || size == 0 {
            return None;
        }

        let mut buf = vec![0u8; size];
        let read = unsafe {
            libc::sysctlbyname(
                key.as_ptr(),
                buf.as_mut_ptr() as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if read != 0 {
            return None;
        }

        let end = buf.iter().position(|&b| b == 0).unwrap_or(size);
        Self::non_empty(String::from_utf8_lossy(&buf[..end]).into_owned())
    }

    #[cfg(any(target_os = "android", target_os = "ios"))]
    fn non_empty(value: String) -> Option<String> {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    }
}
