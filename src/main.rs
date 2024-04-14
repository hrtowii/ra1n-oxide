use rusb::ffi::libusb_control_transfer;
use rusb::{self, UsbContext};
use rusty_libimobiledevice::idevice;
use rusty_libimobiledevice::services::lockdownd;
use std::time::Duration;
use std::thread::sleep;

struct usb_handle {
    vid: u16,
    pid: u16,
    usb_interface: u8,
}
enum HandleOrError {
    Handle(usb_handle),
    Bool(bool),
}
// 0x5ac, 0x1227 -> dfu
// 0x5ac, 0x1281 -> recovery
// 0x5ac, 0x4141 -> pongo

fn find_device(mode: &str, device_descriptor: &rusb::DeviceDescriptor) ->  bool {
    if device_descriptor.vendor_id() != 0x5ac { // just bail if not apple
        return false;
    }
    println!("product ID: 0x{:04x}", device_descriptor.product_id());
    match mode {
        "dfu" => {
            if device_descriptor.product_id() == 0x1227
            {
                println!("Device in DFU found!");
                return true;
            }
        },
        "recovery" => {
            if device_descriptor.product_id() == 0x1281
            {
                println!("Device in recovery found!");
                return true;
            }
        },
        "pongo" => {
            if device_descriptor.product_id() == 0x4141
            {
                println!("Device in pongoOS found!");
                return true;
            }
        },
        _ => {
            println!("Invalid mode");
            return false;
        }
    };
    return false;
}

fn find_device_in_dfu() -> HandleOrError {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_desc = device.device_descriptor().unwrap();
        if find_device("dfu", &device_desc) == true {
            let handle = device.open().unwrap();
            let config_descriptor = device.config_descriptor(0).unwrap();
            let usb_interface = config_descriptor.interfaces().next().unwrap();
            let usb_interface_desc = usb_interface.descriptors().next().unwrap();
            let usb_interface_number = usb_interface_desc.interface_number();
            let usb_handle = usb_handle {
                vid: device_desc.vendor_id(),
                pid: device_desc.product_id(),
                usb_interface: usb_interface_number,
            };
            println!("Vendor ID: 0x{:04x}", usb_handle.vid);
            println!("Product ID: 0x{:04x}", usb_handle.pid);
            return HandleOrError::Handle(usb_handle);
        }
    }
    return HandleOrError::Bool(false);
}

fn find_device_in_recovery() -> bool {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_desc = device.device_descriptor().unwrap();
        if find_device("recovery", &device_desc) == true {
            let handle = device.open().unwrap();
            let config_descriptor = device.config_descriptor(0).unwrap();
            let usb_interface = config_descriptor.interfaces().next().unwrap();
            let usb_interface_desc = usb_interface.descriptors().next().unwrap();
            let usb_interface_number = usb_interface_desc.interface_number();
            let usb_handle = usb_handle {
                vid: device_desc.vendor_id(),
                pid: device_desc.product_id(),
                usb_interface: usb_interface_number,
            };
            // println!("Vendor ID: 0x{:04x}", usb_handle.vid);
            // println!("Product ID: 0x{:04x}", usb_handle.pid);
            return true;
        }
    }
    return false;
}

fn dfu_helper() {
    println!("DFU helper");
}

fn kick_into_recovery() -> bool {
    match find_device_in_dfu() {
        HandleOrError::Bool(false) => {
            println!("No device in DFU, kicking into recovery");
        }
        _ => {}
    }
    let device_list = idevice::get_devices().unwrap();
    let ret = lockdownd::LockdowndClient::new(&device_list[0], "dfu");
    if let Ok(client) = ret {
        lockdownd::LockdowndClient::enter_recovery(&client);
    }
    sleep(Duration::from_secs(10));
    if find_device_in_recovery() {
        println!("Kicked into recovery");
        return true;
    };
    println!("Failed to kick into recovery");
    return false;
}

fn reset_device(usb_handle: &usb_handle) {
    println!("Resetting device");

}

fn heap_fengshui(usb_handle: &usb_handle) {
println!("Stage 1: heap fengshui");
}

fn trigger_uaf(usb_handle: &usb_handle) {
println!("Stage 2: trigger uaf");
}

fn overwrite(usb_handle: &usb_handle) {
println!("Stage 3: overwrite");
}

fn send_payload(usb_handle: &usb_handle) {
println!("stage 3.5: send payload");
}

fn main() {
    if !find_device_in_recovery() {
        if kick_into_recovery() {
            dfu_helper();
        };
    } else if let HandleOrError::Handle(ourhandle) = find_device_in_dfu() {
        println!("Device in DFU mode");
        reset_device(&ourhandle);
        heap_fengshui(&ourhandle);
        trigger_uaf(&ourhandle);
        overwrite(&ourhandle);
        send_payload(&ourhandle);
    } else {
        println!("No device found");
    }
}
