//t503d
//a pseudodriver daemon for the drawing tablet
//of the tenmoon variety
//made in 4 days for a bar of chocolate
#[allow(unused_must_use)]
use nix::{
    unistd::Uid
};
use rusb::{
    Context, 
    Device, 
    UsbContext,
    HotplugBuilder,
    Registration
};
use signal_hook::{
    iterator::Signals,
    consts::{
        SIGINT, 
        SIGTERM
    },
};

use evdev::{
    uinput::VirtualDeviceBuilder,
    AbsoluteAxisCode,
    AbsInfo,
    AttributeSet,
    EventType,
    InputEvent,
    KeyCode,
    KeyEvent,
    UinputAbsSetup
};

use daemonize::Daemonize;

use serde::{
    Serialize,
    Deserialize,
};

use std::{
    io::{
        Write, 
        Read
    },
    time::Duration,
    fs::{
        File,
        create_dir_all
    },
    process::exit,
    thread,
    sync::{
        RwLock,
        Arc
    },
    str::FromStr,
    path::Path
};

const VID: u16 = 0x08f2;
const PID: u16 = 0x6811;
const iface_num: u8 = 2;

/*
static x1: usize = 3; //5
static x2: usize = 2; //4
static y1: usize = 5; //3
static y2: usize = 4; //2
                      */
static x1: usize = 5;
static x2: usize = 4;
static y1: usize = 3;
static y2: usize = 2;

/* ===== CONFIG ===== */

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ConfPen{
    max_x: i32,
    max_y: i32,
    max_pressure: i32,
    resolution_x: i32,
    resolution_y: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ConfSettings{
    swap_axis:  bool,
    flip_y:  bool,
    flip_x:  bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ConfMap{
    x_per:  i32,
    y_per:  i32,
    x_off:  i32,
    y_off:  i32,
}

#[derive(Clone, Debug)]
struct ConfUsable{
    vendor_id: u16,
    product_id: u16,
    pen: ConfPen,
    actions: [Vec<KeyCode>; 6],
    settings: ConfSettings,
    map: ConfMap,
}

impl ConfUsable{
    fn from_parseable(cp: &ConfParseable) -> Result<Self, ()> {
        Ok(ConfUsable{
            vendor_id:  cp.vendor_id.clone(),
            product_id: cp.product_id.clone(),
            pen:        cp.pen.clone(),
            actions:    [
                    parse_to_keycode(cp.actions.button1.clone())?,
                    parse_to_keycode(cp.actions.button2.clone())?,
                    parse_to_keycode(cp.actions.button3.clone())?,
                    parse_to_keycode(cp.actions.button4.clone())?,
                    parse_to_keycode(cp.actions.pen1.clone())?,
                    parse_to_keycode(cp.actions.pen2.clone())?,
            ],
            settings:   cp.settings.clone(),
            map:        cp.map.clone(),
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct ConfParseableButtons{
    button1:    String,
    button2:    String,
    button3:    String,
    button4:    String,
    pen1:       String,
    pen2:       String,
}

#[derive(Clone, Serialize, Deserialize)]
struct ConfParseable {
    vendor_id: u16,
    product_id: u16,
    pen: ConfPen,
    actions: ConfParseableButtons,
    settings: ConfSettings,
    map: ConfMap,
}

impl ConfParseable{
    fn default() -> Self {
        ConfParseable{
            vendor_id:  VID,
            product_id: PID,
            pen:        { ConfPen {
                max_x: 4080,
                max_y: 4080,
                max_pressure: 2047,
                resolution_x: 20,
                resolution_y: 30,
            }},
            actions:    { ConfParseableButtons {
                button1:    String::from("KEY_LEFTCTRL+KEY_Z"),
                button2:    String::from("KEY_LEFTCTRL+KEY_A"),
                button3:    String::from("KEY_C"),
                button4:    String::from("KEY_D"),
                pen1:       String::from("BTN_RIGHT"),
                pen2:       String::from("BTN_MIDDLE"),
            }},
            settings:   { ConfSettings {
                swap_axis:  true,
                flip_y:     false,
                flip_x:     false,
            }},
            map:        { ConfMap {
                x_per:  100,
                y_per:  100,
                x_off:  0,
                y_off:  0,
            }}
        }
    }
}

enum ConfigOption{
    Conf(ConfUsable),
    BadConf(),
    NoConf(),
}

fn found_config(fp: &str) -> ConfigOption {
    let mut file = match File::open(fp) {
        Ok(file) => {file}
        Err(_)   => {return crate::ConfigOption::NoConf()}
    };

    let mut data = String::new();
    match file.read_to_string(&mut data) {
        Ok(_)   => {}
        Err(_)  => {return crate::ConfigOption::BadConf()}
    }
    let parse_data: ConfParseable = match serde_yaml::from_str(&data){
        Ok(data)    => {data}
        Err(_)      => {return crate::ConfigOption::BadConf()}
    };
    let conf_data = match ConfUsable::from_parseable(&parse_data) {
        Ok(data)    => {data}
        Err(_)      => {return crate::ConfigOption::BadConf()}
    };

    //finds and checks config for all operational entries
    //if all good, returns ConfUsable
    //if anything off, ret
    //this includes poorly declared keys and keycombos, which we check by trying a pre-emptive convert to KeyCodes
    //pre-emptive check can work, converting actions as string into an array of vectors consisting of KeyCodes
    //realize as enum?
    
    crate::ConfigOption::Conf(conf_data)
}

fn create_config(fp: &str, conf: &ConfParseable) {
    let fp = Path::new(fp);
    if let Some(parent) = fp.parent() {
        create_dir_all(parent).unwrap();
    }
    let mut file = File::create(fp).unwrap();
    let conf_data = serde_yaml::to_string(conf).unwrap();
    file.write_all(&conf_data.as_bytes());
}


fn config(fp: &str, mut logger: File) -> ConfUsable {

    match found_config(fp){
        crate::ConfigOption::Conf(conf) => {conf}
        crate::ConfigOption::NoConf()   => {
            logger.write_all(b"[NOTE]: No configuration found. Making one.\n"); 
            let conf = ConfParseable::default();
            create_config(fp, &conf);
            ConfUsable::from_parseable(&conf).unwrap()
        }
        crate::ConfigOption::BadConf()  => {
            logger.write_all(b"[ERR]:  Config is bad. Using defaults.\n");
            ConfUsable::from_parseable(&ConfParseable::default()).unwrap()
        }
    }

}

/* ===== THREADS ===== */

struct HotPlugHandler{
    logger: File,
    config: ConfUsable,
}

impl Clone for HotPlugHandler{
    fn clone(&self) -> Self{
        HotPlugHandler{
            logger: self.logger.try_clone().unwrap(),
            config: self.config.clone(),
        }
    }
}

impl Drop for HotPlugHandler {
    fn drop(&mut self) {
    }
}

impl<T: UsbContext + 'static> rusb::Hotplug<T> for HotPlugHandler {
    fn device_arrived(&mut self, device: Device<T>) {
        self.logger.write_all(b"device arrival\n");
        if let Ok(desc) = device.device_descriptor() {
            let vid = desc.vendor_id();
            let pid = desc.product_id();
            self.logger.write_all(b"device identity: VID: ");
            self.logger.write_all(vid.to_string().as_bytes());
            self.logger.write_all(b" PID: ");
            self.logger.write_all(pid.to_string().as_bytes());
            self.logger.write_all(b"\n");
            if vid == VID && pid == PID {
                let mut logclone = self.logger.try_clone().unwrap();
                let mut confclone = self.config.clone();
                thread::spawn(move || {
                    device_runner(device, logclone, confclone);
                });
            }
        }
    }

    fn device_left(&mut self, device: Device<T>) {
        self.logger.write_all(b"device left\n");
        //вообще похуй.
    }
}

impl HotPlugHandler {
    fn new(mut log: File, conf: ConfUsable) -> Self {
        HotPlugHandler{
            logger: log,
            config: conf,
        }
    }
}

fn device_runner<T: UsbContext>(mut device: Device<T>, mut logger: File, cf: ConfUsable) -> rusb::Result<()>{
    
    logger.write_all(b"thread start\n");

    let mut keys = AttributeSet::<KeyCode>::new();
    for i in &cf.actions{
        for e in i{
            keys.insert(*e);
        }
    }
    keys.insert(KeyCode::BTN_TOUCH);

    let abs_setup_x = AbsInfo::new(0, 0, cf.pen.max_x, 0, 0, 0);
    let abs_setup_y = AbsInfo::new(0, 0, cf.pen.max_y, 0, 0, 0);

    let mut virt = VirtualDeviceBuilder::new().unwrap()     //TODO
        .name("t503 virtual driver")
        .with_keys(&keys).unwrap()
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, abs_setup_x)).unwrap()
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, abs_setup_y)).unwrap()
        .build()
        .unwrap();

    let mut touch_event: [KeyEvent; 2] = [KeyEvent::new(KeyCode(KeyCode::KEY_0.0), 0); 2];
    touch_event[0] = KeyEvent::new(KeyCode(KeyCode::BTN_TOUCH.0), 1);
    touch_event[1] = KeyEvent::new(KeyCode(KeyCode::BTN_TOUCH.0), 0);

    let mut config = device.active_config_descriptor()?;
    let mut iface = match config.interfaces().nth(iface_num as usize) {
        Some(ifc) => {ifc}
        None => {logger.write_all(b"No such interface!\n"); panic!("FUCK!?")}
    };
    let descriptor = iface.descriptors().next().unwrap();
    let endpoint = descriptor.endpoint_descriptors().next().unwrap();

    let mut handle = device.open()?;
    handle.reset();

    let mut kernel_driver = false;
    if handle.kernel_driver_active(iface_num)? {
        handle.detach_kernel_driver(iface_num)?;    //if this doesn't work, copy what calico did
        kernel_driver = true;
    }

    let max_x = cf.pen.max_x * cf.settings.flip_x as i32;
    let max_y = cf.pen.max_y * cf.settings.flip_y as i32;
    let x_per = cf.map.x_per / 100;
    let y_per = cf.map.y_per / 100;
    let x_off = cf.map.x_off * cf.pen.max_x / 100;
    let y_off = cf.map.y_off * cf.pen.max_y / 100;

    let mut buffer: [u8; 8] = [1; 8];
    loop{
        buffer.fill(0);
        let anything = match handle.read_bulk(endpoint.address(), &mut buffer, Duration::from_millis(0)) {
            Err(_) => {break;}
            Ok(data) => {data}
        };
        if anything != 0 {
            logger.write_all(&format!("bytelength: {:?}\nbuffer: {:?}\n", anything, buffer).as_bytes());
            if buffer[0] == 2 { //buttons
                logger.write_all(b"button press registered\n");
                let mut event_code = 6;
                let mut pressed = 1;
                if      buffer[1] == 2 {    //btn 1
                    event_code = 0;
                }
                else if buffer[1] == 4 {    //btn 2
                    event_code = 1;
                }
                else if buffer[3] == 44 {   //btn 3
                    event_code = 2;
                }
                else if buffer[3] == 43 {   //btn 4
                    event_code = 3;
                }
                else if buffer[1] == 1 && buffer[3] == 28 { //pen 1
                    event_code = 4;
                }
                else if buffer[1] == 1 && buffer[3] == 29 { //pen 2
                    event_code = 5;
                }
                else {
                    pressed = 0;
                }
                if event_code != 6 { 
                    for e in cf.actions[event_code].clone(){
                        virt.emit(&[*KeyEvent::new(e, pressed)]).unwrap();
                    }
                }
            }
            else if buffer[1] == 192 || buffer[1] == 193 { //pen
                logger.write_all(b"touch movement registered\n");
                let pen_x = (max_x - (buffer[x1] as i32 * 255 + buffer[x2] as i32) ).abs() * x_per + x_off;
                let pen_y = (max_y - (buffer[y1] as i32 * 255 + buffer[y2] as i32) ).abs() * y_per + y_off;
                let pen_pressure=   buffer[7] as i32 * 255 + buffer[6] as i32;
                let pen_x_event =           InputEvent::new_now(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, pen_x); 
                let pen_y_event =           InputEvent::new_now(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, pen_y);
                let pen_pressure_event =    InputEvent::new_now(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_PRESSURE.0, pen_pressure);
                virt.emit(&[pen_x_event, pen_y_event, pen_pressure_event, *touch_event[(buffer[1] - 192) as usize]]).unwrap();
            }

        }
    }

    logger.write_all(b"a handling thread was killed\n");

    /*
    if kernel_driver {
        handle.attach_kernel_driver(iface_num)?;
    }
    this does nothing because the device is errored (disconnected)
    */
    
    Ok(())
}

/* ===== SIGNALS ===== */

fn signal_handler() -> Result<(), Box<dyn std::error::Error>> {

    let mut signals = Signals::new(&[SIGTERM, SIGINT]).unwrap();

    thread::spawn(move || {
        for sig in signals.forever(){
            match sig {
                SIGINT | SIGTERM => {
                    //TODO: Arc<RwLock> every thread and reattach kernel drivers before exiting!
                    exit(0);
                }
                _ => {}
            }
        }
    });

    Ok(())
}

/* ===== EVDEV ===== */

fn parse_to_keycode(data: String) -> Result<Vec<KeyCode>, ()> {
    let mut result = Vec::new();   
    for e in data.split("+"){
        result.push(
            match KeyCode::from_str(e) {
                Ok(code) => {code}
                Err(_)   => {return Err(())}
            }
        );
    }
    Ok(result)
}

/* ===== OTHER ===== */

fn enter_context(mut context: Context, mut log: File, config: ConfUsable) -> rusb::Result<()>{

    //handles signals
    signal_handler();

    //creates callbacky, for matchies creates a thread (does the inputty)
    let mut reg: Option<Registration<Context>> = Some(
        HotplugBuilder::new()
            .enumerate(true)
            .register(&context, Box::new(HotPlugHandler::new(log.try_clone().unwrap(), config)))?,
    );

    //spin forever
    loop {
        context.handle_events(None).unwrap();
    }

    log.write_all(b"err: exiting main loop\n");

    return Ok(())
}

fn root() -> bool {
    Uid::effective().is_root()
}

fn main() {

    //root check
    if !root(){
        eprintln!("Run me as root, or I won't work!");
        return;
    }

    let mut log = File::create("t503d.log").unwrap(); //TODO put into /etc/t503d

    let mut fp = String::from("/etc/t503d/conf.yaml");

    let conf = config(&fp,log.try_clone().unwrap());
    log.write_all(format!("Loaded Config: \n{:?}\n", conf).as_bytes());

    let daemonize = Daemonize::new().working_directory("/").umask(0o027);
    daemonize.start();

    if !rusb::has_hotplug() {
        log.write_all(b"[ERR]:  daemon launch failure: rusb has no hotplug option.\n");
    } else {
        match Context::new() {
            Ok(mut context) => {enter_context(context, log, conf);}
            Err(_) => {log.write_all(b"[ERR]:  daemon launch failure: libusb inaccessible.\n");}
        }
    }

}
