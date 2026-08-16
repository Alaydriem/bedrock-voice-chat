use rodio::cpal::traits::{DeviceTrait, HostTrait};

#[test]
fn get_cpal_hosts() {
    let hosts = crate::audio::device::AudioDeviceEnumerator::get_cpal_hosts();
    match hosts {
        Ok(hosts) => {
            // On Windows we should have both AISO and WASAPI drivers
            #[cfg(target_os = "windows")]
            {
                assert_eq!(2, hosts.len());
                assert_eq!("Wasapi", hosts.get(0).unwrap().id().name());
                assert_eq!("Asio", hosts.get(1).unwrap().id().name());
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
                assert_eq!("AAudio", hosts.get(0).unwrap().id().name());
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
    // The contract under test is that device enumeration succeeds without
    // erroring. The device count is NOT asserted: headless CI runners have no
    // audio hardware, so an empty list is a valid result there.
    let devices = crate::audio::device::AudioDeviceEnumerator::get_devices();
    match devices {
        Ok(devices) => {
            for (host, device_list) in devices.iter() {
                for device in device_list {
                    println!("[{}] {} {}", host, device.io.store_key(), device.name);
                }
            }
        }
        Err(()) => {
            panic!("device enumeration must not error");
        }
    }

    println!("------");
    let hosts = crate::audio::device::AudioDeviceEnumerator::get_cpal_hosts().unwrap();
    for host in hosts {
        match host.input_devices() {
            Ok(devices) => {
                for device in devices {
                    let name = match device.description() {
                        Ok(desc) => desc.name().to_string(),
                        Err(e) => {
                            println!("{}", e.to_string());
                            continue;
                        }
                    };

                    println!("[{}] {}", host.id().name(), name);
                }
            }
            Err(_) => {}
        }
    }
}
