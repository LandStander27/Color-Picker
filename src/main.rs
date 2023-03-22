#![windows_subsystem="windows"]
#![allow(non_upper_case_globals)]

use std::process::exit;
use scrap::{Capturer, Display, Frame};
use std::thread;
use std::time::Duration;
use std::io::ErrorKind::WouldBlock;
use enigo::Enigo;
use inputbot::MouseButton::*;
use serde_derive::{Deserialize, Serialize};

use msgbox::IconType;

use std::ptr;
use winapi::um::wincon::{GetConsoleWindow, FreeConsole};
use winapi::um::winuser::{ShowWindow, SW_HIDE, SW_SHOW};
use tray_icon::{
	menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
	TrayEvent, TrayIconBuilder, icon::Icon,
};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use colored::Colorize;
use winrt_notification::{Duration as win_duration, Sound, Toast};

use winreg::enums::*;
use winreg::RegKey;
use std::path::Path;

#[link(name = "kernel32")]
extern "system" {
	fn AllocConsole() -> isize;
}

#[derive(Serialize, Deserialize, Debug)]
struct Name {
	value: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Color {
	name: Name
}

fn fix_buffer(buffer: Frame, w: u32, h: u32) -> Vec<u8> {
	// let mut flipped: Vec<u8> = Vec::new();

	// let mut current: usize = 0;

	// for x in 0..w {
	// 	for y in 0..h {
	// 		flipped.push(buffer[current+2]);
	// 		flipped.push(buffer[current+1]);
	// 		flipped.push(buffer[current]);
	// 		flipped.push(buffer[current+3]);
	// 		current += 4;
	// 	}
	// }

	// // for i in flipped.clone() {
	// // 	println!("{}", i);
	// // }

	// repng::encode(
	// 	File::create("screenshot.png").unwrap(),
	// 	w,
	// 	h,
	// 	&flipped,
	// ).unwrap();

        let mut bitflipped = Vec::with_capacity(w as usize * h as usize * 4);
        let stride = buffer.len() / h as usize;

        for y in 0..h {
		for x in 0..w {
			let i = stride * y as usize + 4 * x as usize;
			bitflipped.extend_from_slice(&[
				buffer[i + 2],
				buffer[i + 1],
				buffer[i],
				255,
			]);
		}
        }

	return bitflipped;

}

fn get_pixel(buffer: Vec<u8>, height: u32, x: u32, y: u32) -> Vec<u8> {
	let i = buffer.len() as u32 / height * y + 4 * x;
	return Vec::from([buffer[i as usize], buffer[i as usize + 1], buffer[i as usize + 2]]);
}

fn get_color_name(code: String) -> Result<String, String> {
	let resp = reqwest::blocking::get(format!("https://www.thecolorapi.com/id?hex={}", code));
	let name: String = match resp {
		Ok(j) => {

			// let json = j.json::<HashMap<String, String>>().unwrap();
			// json["name"].clone()
			let parsed = j.json::<Color>().unwrap();
			parsed.name.value
		},
		Err(e) => {
			return Err(e.to_string());
		}
	};
	return Ok(name)
}

fn find_color() {
	'outer: loop {

		if LeftButton.is_pressed() {
			let (x, y): (i32, i32) = Enigo::mouse_location();


			let display = Display::primary().unwrap();
			let mut capturer = Capturer::new(display).unwrap();
			let (w, h) = (capturer.width(), capturer.height());
		
			if x > w as i32 || x < 0 || y > h as i32 || y < 0 {
				println!("{}{}, {}", "Invalid mouse position: ".red(), x, y);
				return;
			}

			match capturer.frame() {
				Ok(_) => {},
				Err(_) => {}
			}
			thread::sleep(Duration::new(1, 0)/60);

			loop {
	    
				// buffer = match capturer.frame() {
				// 	Ok(buffer) => buffer,
				// 	Err(error) => {
				// 		if error.kind() == WouldBlock {
				// 			thread::sleep(one_frame);
				// 			continue;
				// 		} else {
				// 			println!("Error: {}", error);
				// 			exit(1);
				// 		}
				// 	}
				// };
		
				println!("{}", "Attempting to take screenshot".yellow());

				match capturer.frame() {
					Ok(buffer) => {

						let pixel = get_pixel(fix_buffer(buffer, w as u32, h as u32), h as u32, x as u32, y as u32);
						// let hexcode: String = format!("{:x}{:x}{:x}", pixel[0], pixel[1], pixel[2]);
						let hexcode: Vec<String> = Vec::from([format!("{:x}", pixel[0]), format!("{:x}", pixel[1]), format!("{:x}", pixel[2])]);
				
						let mut formatted: String = String::from("");
						for i in hexcode {
							if i.len() == 1 {
								formatted.push_str("0");
							}
							formatted.push_str(i.as_str());
						}
						
						let res = get_color_name(formatted.clone());

						let name = match res {
							Ok(r) => r,
							Err(e) => {
								println!("{}{}", "Could not retrieve color name: ".red(), e);
								String::from("")
							}
						};

						if let Err(e) = msgbox::create("Color match", format!("Hexcode: {}\nColor name: {}", formatted, name).as_str(), IconType::Info) {
							println!("{}{}", "Could not show messagebox: ".red(), e);
						}
	
						

						break 'outer;
						
					}
					// Err(error) => {
					// 	if error.kind() == WouldBlock {
		
					// 	} else {
							// println!("Error: {}", error);
							// exit(1);
					// 	}
					// }
					Err(e) if e.kind() == WouldBlock => {
						thread::sleep(Duration::new(1, 0)/60);
					},
					Err(e) => {
						println!("{}{}", "Error: ".red(), e);
						exit(1);
					}
				};

			}
		}


	};
}

fn load_icon(path: &std::path::Path) -> tray_icon::icon::Icon {
	let (icon_rgba, icon_width, icon_height) = {
		let image = image::open(path)
			.expect("Failed to open icon path".red().to_string().as_str())
			.into_rgba8();
		let (width, height) = image.dimensions();
		let rgba = image.into_raw();
		(rgba, width, height)
	};
	tray_icon::icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)
		.expect("Failed to open icon".red().to_string().as_str())
}

static mut console_visible: bool = true;

fn toggle_console(value: bool) {
	if value == true {
		let window = unsafe { GetConsoleWindow() };
		if window != ptr::null_mut() {
			unsafe { ShowWindow(window, SW_SHOW); }
		} else {
			println!("{}", "Could not get window handle".red());
		}
	} else {
		let window = unsafe { GetConsoleWindow() };
		if window != ptr::null_mut() {
			unsafe { ShowWindow(window, SW_HIDE); }
		} else {
			println!("{}", "Could not get window handle".red());
		}
	}
}

fn check_autostart() -> Result<bool, String> {
	let hkcu = RegKey::predef(HKEY_CURRENT_USER);
	let path = Path::new("Software").join("Microsoft").join("Windows").join("CurrentVersion").join("Run");
	let (key, _disp) = hkcu.create_subkey(&path).unwrap();

	match key.get_value::<String, &str>("ColorPicker") {
		Ok(g) => {
			if g == std::env::current_exe().unwrap().display().to_string() {
				return Ok(true);
			} else {
				return Ok(false);
			}
		},
		Err(e) => {
			if e.to_string().contains("The system cannot find the file specified") {
				return Ok(false);
			}
			return Err("Error: ".to_string() + e.to_string().as_str());
		}
	}

}

fn main() {

	unsafe { AllocConsole() };
	println!("Allocated console.");
	thread::sleep(Duration::from_millis(1250));

	println!("Checking for autostart.");
	let autostart: bool = check_autostart().unwrap();


	println!("Loading icon file.");
	println!("{}", "Assuming icon file exists.".yellow());
	let icon: Icon = load_icon(std::path::Path::new("icon.png"));

	println!("Creating system tray menu.");
	let tray_menu = Menu::new();
	let event_loop = EventLoopBuilder::new().build();

	let exit_button = MenuItem::new("Exit", true, None);

	let activate_button = MenuItem::new("Activate", true, None);
	let autostart_button = MenuItem::new(if autostart { "Disable autostart" } else { "Enable autostart" }, true, None);

	let toggle_button = MenuItem::new("Toggle console", true, None);

	tray_menu.append_items(&[
		&toggle_button,
		&activate_button,
		&PredefinedMenuItem::separator(),
		&autostart_button,
		&PredefinedMenuItem::separator(),
		&exit_button,
	]);

	let _tray_icon = TrayIconBuilder::new()
		.with_menu(Box::new(tray_menu))
		.with_tooltip("Color finder")
		.with_icon(icon)
		.build()
		.unwrap();

	println!("Initializing channel listeners.");

	let menu_channel = MenuEvent::receiver();
	let tray_channel = TrayEvent::receiver();
	
	println!("Starting minimize thread.");
	thread::spawn(|| {
		// toggle_console(true);
		println!("Minimizing to system tray in 2 seconds...");
		thread::sleep(Duration::from_millis(2000));
		toggle_console(false);
		unsafe { console_visible = false; }
	});

	println!("Starting event thread.");

	event_loop.run(move |_event, _, control_flow| {
		*control_flow = ControlFlow::Poll;

		if let Ok(event) = menu_channel.try_recv() {
			if event.id == exit_button.id() {
				unsafe { FreeConsole(); }
				*control_flow = ControlFlow::Exit;
				exit(0);


			}
			
			if event.id == toggle_button.id() {
				println!("Attempting to find console window");
				if unsafe { console_visible } {
					// unsafe { FreeConsole(); }
					toggle_console(false);
				} else {
					// unsafe { AllocConsole(); }
					toggle_console(true);
				}
				unsafe { console_visible = !console_visible }
			}

			if event.id == activate_button.id() {
				println!("Attempting to find color");

				let res = Toast::new(Toast::POWERSHELL_APP_ID)
					.title("Color picker")
					.text1("On the next click you will find your color!")
					.sound(Some(Sound::SMS))
					.duration(win_duration::Short)
					.show();

				if let Err(e) = res {
					println!("{}{}", "Could not show toast: ".red(), e);
				}

				find_color();
			}

			if event.id == autostart_button.id() {
				if autostart_button.text() == "Enable autostart" {
					autostart_button.set_text("Disable autostart");

					let hkcu = RegKey::predef(HKEY_CURRENT_USER);
					let path = Path::new("Software").join("Microsoft").join("Windows").join("CurrentVersion").join("Run");
					let (key, _disp) = hkcu.create_subkey(&path).unwrap();
				
					if let Err(e) = key.set_value("ColorPicker", &std::env::current_exe().unwrap().display().to_string().as_str()) {
						println!("Error: Unable to set registry key: {}", e);
						return;
					}

				} else {
					autostart_button.set_text("Enable autostart");

					let hkcu = RegKey::predef(HKEY_CURRENT_USER);
					let path = Path::new("Software").join("Microsoft").join("Windows").join("CurrentVersion").join("Run");
					let (key, _disp) = hkcu.create_subkey(&path).unwrap();
				
					if let Err(e) = key.delete_value("ColorPicker") {
						println!("Error: Unable to delete registry key: {}", e);
						return;
					}

				}
			}
		}

		if let Ok(event) = tray_channel.try_recv() {
			if event.event == tray_icon::ClickEvent::Left {
				println!("{}", "Assuming click means to find color.".yellow());

				println!("Attempting to find color");

				let res = Toast::new(Toast::POWERSHELL_APP_ID)
					.title("Color picker")
					.text1("On the next click you will find your color!")
					.sound(Some(Sound::SMS))
					.duration(win_duration::Short)
					.show();

				if let Err(e) = res {
					println!("{}{}", "Could not show toast: ".red(), e);
				}

				find_color();

			}
		}
			


	});


}
