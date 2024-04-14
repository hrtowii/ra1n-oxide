use rusb::ffi::libusb_control_transfer;
use rusb::{self, DeviceDescriptor, DeviceHandle, UsbContext};
use rusty_libimobiledevice::idevice;
use rusty_libimobiledevice::services::lockdownd;
use std::thread::sleep;
use std::time::Duration;

// 0x5ac, 0x1227 -> dfu
// 0x5ac, 0x1281 -> recovery
// 0x5ac, 0x4141 -> pongo

fn find_device(mode: &str, device_descriptor: &DeviceDescriptor) -> bool {
    if device_descriptor.vendor_id() != 0x5ac {
        // just bail if not apple
        return false;
    }
    // println!("product ID: 0x{:04x}", device_descriptor.product_id());
    match mode {
        "dfu" => {
            if device_descriptor.product_id() == 0x1227 {
                println!("Device in DFU found!");
                return true;
            }
        }
        "recovery" => {
            if device_descriptor.product_id() == 0x1281 {
                println!("Device in recovery found!");
                return true;
            }
        }
        "pongo" => {
            if device_descriptor.product_id() == 0x4141 {
                println!("Device in pongoOS found!");
                return true;
            }
        }
        _ => {
            println!("Invalid mode");
            return false;
        }
    };
    return false;
}

fn find_device_in_dfu() -> Option<rusb::Device<rusb::Context>> {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_desc = device.device_descriptor().unwrap();
        if find_device("dfu", &device_desc) == true {
            return Some(device.clone());
        }
    }
    return None;
}

fn find_device_in_recovery() -> bool {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_desc = device.device_descriptor().unwrap();
        if find_device("recovery", &device_desc) == true {
            return true;
        }
    }
    return false;
}

fn dfu_helper() {
    println!("DFU helper");
}

fn send_command_to_recovery(usb_handle: &rusb::DeviceHandle<rusb::Context>, command: &str) {
    if command.len() <= 0x100 && command.len() > 1 {
        let unsafe_handle = usb_handle.as_raw();
        unsafe {
            libusb_control_transfer(
                unsafe_handle,
                0x40,
                1,
                0,
                0,
                command.as_ptr() as *mut u8,
                (command.len() + 1).try_into().unwrap(),
                10,
            );
        }
    } else {
        println!("Invalid command length");
    }
}

fn kick_into_recovery() -> bool {
    match find_device_in_dfu() {
        None => {
            println!("No device in DFU, kicking into recovery");
        }
        _ => {
            println!("Device in DFU, exiting");
            return false;
        }
    }
    let device_list = idevice::get_devices().unwrap();
    let ret = lockdownd::LockdowndClient::new(&device_list[0], "dfu");
    if let Ok(client) = ret {
        let _ = lockdownd::LockdowndClient::enter_recovery(&client);
    }
    // TODO: add another thread to check if device is in recovery, because this is blocking
    let mut counter = 0;
    while find_device_in_recovery() == false {
        sleep(Duration::from_secs(1));
        counter += 1;
        if counter > 30 {
            break;
        }
    }
    if find_device_in_recovery() {
        return true;
    }
    println!("Failed to kick into recovery");
    return false;
}

fn reset_device(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    println!("Resetting device");
}

fn heap_fengshui(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    println!("Stage 1: heap fengshui");
}

fn send_abort(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    let unsafe_handle = usb_handle.as_raw();
    unsafe {
        libusb_control_transfer(
            unsafe_handle,
            0x21,
            0x4,
            0,
            0,
            std::ptr::null_mut(),
            0,
            0
        );
    }
    // send_usb_control_request_no_data(handle, 0x21, 0x4, 0, 0, 0, NULL);
}

fn trigger_uaf(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    //     1. Start a **control request transfer** with **data phase**
    // 	        1. Interrupt the transfer halfway
    //     2. Issue a **DFU abort** (0x21, 4), which frees the USB buffer
    // 	        1. DFU abort will cause us to reenter, which **restarts the USB stack and reallocates our buffer**
    //     3. Finish the interrupted transfer.
    // 	        1. **Send data phase packets** once DFU is re-entered.
    //     4. The data will be `memcpy`d on top of the freed pointer.

    println!("Stage 2: trigger uaf");

    send_abort(&usb_handle);
}

fn overwrite(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    println!("Stage 3: overwrite");
}

fn send_payload(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    println!("stage 3.5: send payload");
}

fn main() {
    if let Some(device) = find_device_in_dfu() {
        // println!("Device in DFU mode");
        let device_handle = device.open().unwrap();
        reset_device(&device_handle);
        heap_fengshui(&device_handle);
        trigger_uaf(&device_handle);
        overwrite(&device_handle);
        send_payload(&device_handle);
    } else if !find_device_in_recovery() {
        if kick_into_recovery() {
            dfu_helper();
        }
    } else {
        // println!("Device in recovery mode");
        dfu_helper();
    }
}
