use rusb::ffi::{libusb_control_transfer, libusb_error_name, libusb_strerror};
use rusb::{self, DeviceDescriptor, UsbContext};
use rusty_libimobiledevice::idevice;
use rusty_libimobiledevice::services::lockdownd;
use std::thread::sleep;
use std::time::Duration;
use tokio;

// MARK: constants
pub const DFU_DNLOAD: u8 = 1;
pub const DFU_UPLOAD: u8 = 2;
pub const DFU_GETSTATUS: u8 = 3;
pub const DFU_CLRSTATUS: u8 = 4;
pub const DFU_GETSTATE: u8 = 5;
pub const DFU_ABORT: u8 = 6;
pub const DFU_FILE_SUFFIX_LENGTH: usize = 16;
pub const EP0_MAX_PACKET_SIZE: u16 = 0x40;
pub const DFU_MAX_TRANSFER_SIZE: u16 = 0x800;
pub const DFU_STATUS_OK: u8 = 0;
pub const DFU_STATE_MANIFEST_SYNC: u8 = 6;
pub const DFU_STATE_MANIFEST: u8 = 7;
pub const DFU_STATE_MANIFEST_WAIT_RESET: u8 = 8;

// USB constants
pub const USB_TIMEOUT: u32 = 10;

// 0x5ac, 0x1227 -> dfu
// 0x5ac, 0x1281 -> recovery
// 0x5ac, 0x4141 -> pongo

// MARK: device detection
async fn find_apple_device() -> Option<rusb::DeviceHandle<rusb::Context>> {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_handle = device.open().unwrap();
        let device_descriptor = device_handle.device().device_descriptor().unwrap();
        if device_descriptor.vendor_id() == 0x5ac && device_descriptor.vendor_id() != 0x1227 && device_descriptor.vendor_id() != 0x1281 && device_descriptor.vendor_id() != 0x4141 {
            sleep(Duration::from_millis(800));
            return Some(device_handle);
        }
    }
    return None;
}

async fn find_device(mode: &str, device_descriptor: &DeviceDescriptor) -> bool {
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

async fn find_device_in_dfu() -> Option<rusb::Device<rusb::Context>> {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_desc = device.device_descriptor().unwrap();
        if find_device("dfu", &device_desc).await == true {
            return Some(device.clone());
        }
    }
    return None;
}

async fn find_device_in_recovery() -> Option<rusb::Device<rusb::Context>> {
    let context = rusb::Context::new().unwrap();
    let device_list = context.devices().unwrap();
    for device in device_list.iter() {
        let device_desc = device.device_descriptor().unwrap();
        if find_device("recovery", &device_desc).await == true {
            return Some(device.clone());
        }
    }
    return None;
}

fn timer(mut seconds: u64, what_to_say: &str) {
    while seconds > 0 {
        println!("\r{} {}", seconds, what_to_say);
        sleep(Duration::from_secs(1));
        seconds -= 1;
    }
}

/*
if home button:
hold power and home button for 4 seconds
hold home button for 10 seconds

if no home button:
hold voldown + side for 4 seconds
release side buttom, keep holding voldown for 10 seconds

cpid of no home:
#define NOHOME (cpid == 0x8015 || (cpid == 0x8010 && (bdid == 0x08 || bdid == 0x0a || bdid == 0x0c || bdid == 0x0e)))

Example serial number: 
CPID:8010 <- Get by searching for CPID: and then getting the next 4 characters
 CPRV:11 CPFM:03 SCEP:01 BDID:08 <- Get by searching for BDID: and then getting the next 2 characters
 ECID:000269E20846003A IBFL:3C SRTG:[iBoot-2696.0.0.1.33]
 */

fn get_cpid_from_serial(serial: &str) -> &str {
    let cpid_index = serial.find("CPID:").unwrap();
    // println!("CPID index: {}", cpid_index);
    let cpid = &serial[cpid_index + 5..cpid_index + 9]; // chatgpt did this for me please idk
    // println!("CPID: {}", cpid);
    return &cpid;
}

fn get_bdid_from_serial(serial: &str) -> &str {
    let bdid_index = serial.find("BDID:").unwrap();
    // println!("BDID index: {}", bdid_index);
    let bdid = &serial[bdid_index + 5..bdid_index + 7]; // chatgpt did this for me please idk
    // println!("BDID: {}", bdid);
    return &bdid;
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
                USB_TIMEOUT,
            );
        }
    } else {
        println!("Invalid command length");
    }
}

// MARK: dfu helper
fn dfu_helper(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    let device_descriptor = usb_handle.device().device_descriptor().unwrap();
    let serial_number = usb_handle.read_serial_number_string_ascii(&device_descriptor).unwrap();
    // println!("Serial number: {}", serial_number);
    let cpid = get_cpid_from_serial(&serial_number).parse::<u16>().unwrap();
    let bdid = get_bdid_from_serial(&serial_number).parse::<u16>().unwrap();
    let is_home_button = (cpid == 0x8015 || (cpid == 0x8010 && (bdid == 0x08 || bdid == 0x0a || bdid == 0x0c || bdid == 0x0e)));

    println!("Press any character when you are ready to enter DFU");
    std::io::stdin().read_line(&mut String::new()).unwrap();

    timer(3, "Get ready...");
    if (is_home_button) {
        timer(4, "Hold home + power button");
    } else {
        timer(4, "Hold volume down + side button");
    }

    send_command_to_recovery(usb_handle, "setenv auto-boot true");
    sleep(Duration::from_millis(100));
    send_command_to_recovery(usb_handle, "saveenv");
    sleep(Duration::from_millis(100));
    send_command_to_recovery(usb_handle, "reboot");

    if (is_home_button) {
        timer(10, "Hold down home button only");

    } else {
        timer(10, "Hold down volume button only")
    }
}

async fn kick_into_recovery() -> bool {
    match find_device_in_dfu().await {
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
    if find_device_in_recovery().await.is_some() {
        return true;
    }
    println!("Failed to kick into recovery");
    return false;
}

// MARK: usb stuff
fn reset_device(usb_handle: &rusb::DeviceHandle<rusb::Context>) {
    println!("Resetting device for checkm8");
    let unsafe_handle = usb_handle.as_raw();
    unsafe {
        let mut ret = libusb_control_transfer(
            unsafe_handle,
             0x21,
              DFU_DNLOAD,
              0,
              0,
              std::ptr::null_mut(),
              DFU_FILE_SUFFIX_LENGTH.try_into().unwrap(),
              USB_TIMEOUT,
             );
        // send_usb_control_request_no_data(handle, 0x21, DFU_DNLOAD, 0, 0, DFU_FILE_SUFFIX_LENGTH, &transferRet);
        
    // Send zero length packet to end existing transfer

    // Request image validation like we are about to boot it
    
    // Start a new DFU transfer
    ret = libusb_control_transfer(unsafe_handle,
         0x21,
          DFU_DNLOAD,
           0,
            0,
             std::ptr::null_mut(),
              DFU_FILE_SUFFIX_LENGTH.try_into().unwrap(),
               USB_TIMEOUT
            ); 

    // Ready
    // return true;
    }
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

#[tokio::main]
async fn main() {
    let find_device_in_dfu_task = find_device_in_dfu();
    let find_device_in_recovery_task = find_device_in_recovery();
    let find_apple_device_task = find_apple_device();

    tokio::select! {
        Some(device) = find_device_in_dfu_task => {
            let device_handle = device.open().unwrap();
            reset_device(&device_handle);
            heap_fengshui(&device_handle);
            trigger_uaf(&device_handle);
            overwrite(&device_handle);
            send_payload(&device_handle);
        }
        Some(device) = find_device_in_recovery_task => {
            let device_handle = device.open().unwrap();
            dfu_helper(&device_handle);
        }
        Some(device) = find_apple_device_task => {
            kick_into_recovery().await;
            dfu_helper(&device);
        }
        else => {
            // Handle the case where none of the tasks succeed
            println!("Device detection failed.");
        }
    }
}
