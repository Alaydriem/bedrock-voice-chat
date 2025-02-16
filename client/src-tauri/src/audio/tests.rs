use cpal::traits::{DeviceTrait, HostTrait};

#[test]
fn get_cpal_hosts() {
    let hosts = crate::audio::device::get_cpal_hosts();
    match hosts {
        Ok(hosts) => {
            // On Windows we should have both AISO and WASAPI drivers
            #[cfg(target_os = "windows")]
            {
                assert_eq!(2, hosts.len());
                assert_eq!("WASAPI", hosts.get(0).unwrap().id().name());
                assert_eq!("ASIO", hosts.get(1).unwrap().id().name());
            }

            // On MacOS we should only have CoreAudio
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            {
                assert_eq!(1, hosts.len());
                assert_eq!("CoreAudio", hosts.get(0).unwrap().id().name());
            }

            #[cfg(any(target_os = "android"))]
            {
                assert_eq!(1, hosts.len());
                assert_eq!("Oboe", hosts.get(0).unwrap().id().name());
            }
        }
        Err(e) => {
            assert_eq!(
                e.to_string(),
                "Retrieving hosts did not return an error".to_string()
            );
        }
    }
}

#[test]
fn get_devices() {
    let devices = crate::audio::device::get_devices();
    match devices {
        Ok(devices) => {
            for (host, device_list) in devices.iter() {
                // We should always have _something_
                assert_ne!(0, device_list.len());
                for device in device_list {
                    println!("[{}] {} {}", host, device.io.to_string(), device.name);
                }
            }
        }
        Err(e) => {
            assert_eq!(
                "".to_string(),
                "No devices available on online hosts".to_string()
            );
        }
    }

    println!("------");
    let hosts = crate::audio::device::get_cpal_hosts().unwrap();
    for host in hosts {
        match host.input_devices() {
            Ok(devices) => {
                for device in devices {
                    let name = match device.name() {
                        Ok(name) => name,
                        Err(e) => {
                            println!("{}", e.to_string());
                            continue;
                        }
                    };

                    println!("[{}] {}", host.id().name(), name);
                }
            }
            Err(e) => {}
        }
    }
}
