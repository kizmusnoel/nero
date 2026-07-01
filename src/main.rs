#![no_main]
#![no_std]

mod disk;

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;
use spin::Mutex;
use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::{BltOp, BltPixel, GraphicsOutput};
use uefi::proto::console::text::{Key, ScanCode};

static HOSTNAME: Mutex<String> = Mutex::new(String::new());
static COMMAND_HISTORY: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

fn maximize_console() {
    system::with_stdout(|stdout| {
        let best = stdout
            .modes()
            .max_by_key(|m| m.rows() * m.columns())
            .unwrap();

        stdout.set_mode(best).unwrap();
        stdout.clear().unwrap();
    });
}

/// Clears the framebuffer (removes OEM splash/logo) without stealing
/// exclusive ownership from the text console.
fn clear_screen() -> uefi::Result {
    let handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;

    let mut gop = unsafe {
        boot::open_protocol::<GraphicsOutput>(
            uefi::boot::OpenProtocolParams {
                handle,
                agent: boot::image_handle(),
                controller: None,
            },
            uefi::boot::OpenProtocolAttributes::GetProtocol,
        )?
    };

    let (width, height) = gop.current_mode_info().resolution();

    gop.blt(BltOp::VideoFill {
        color: BltPixel::new(0, 0, 0),
        dest: (0, 0),
        dims: (width, height),
    })?;

    // reset cursor to top-left
    system::with_stdout(|stdout| {
        stdout.clear().unwrap();
    });

    Ok(())
}

/// Reads a line of input, echoing characters as typed.
/// Returns `Some(line)` on Enter, or `None` if Escape was pressed.
fn read_line() -> Option<String> {
    let mut buffer = String::new();

    loop {
        uefi::system::with_stdin(|stdin| {
            let _ = boot::wait_for_event(&mut [stdin.wait_for_key_event().unwrap()]);
        });

        let key = uefi::system::with_stdin(|stdin| stdin.read_key());

        match key {
            Ok(Some(Key::Printable(ch))) => {
                let c: char = char::from(ch);

                match c {
                    '\r' | '\n' => {
                        COMMAND_HISTORY.lock().push_back(buffer.clone());
                        uefi::println!();
                        return Some(buffer);
                    }
                    '\u{8}' => {
                        if buffer.pop().is_some() {
                            uefi::print!("\u{8} \u{8}");
                        }
                    }
                    _ => {
                        buffer.push(c);
                        uefi::print!("{}", c);
                    }
                }
            }
            Ok(Some(Key::Special(ScanCode::ESCAPE))) => {
                uefi::println!();
                return None;
            }
            Ok(Some(Key::Special(ScanCode::UP))) => {
                for _ in 0..buffer.len() {
                    uefi::print!("\u{8} \u{8}");
                }
                buffer = String::new();

                if let Some(last) = COMMAND_HISTORY.lock().pop_back() {
                    buffer = last.clone();
                    uefi::print!("{}", last);
                }
            }
            _ => {}
        }
    }
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    maximize_console();

    *HOSTNAME.lock() = String::from("local");

    if let Err(e) = clear_screen() {
        uefi::println!("clear_screen failed: {:?}", e);
    }

    uefi::println!("WELCOME TO NERO (New Rust-based Operating System)!");
    uefi::println!("(Press Esc at any time to exit to the next boot option)");

    loop {
        uefi::print!("\nsuper@{} > ", HOSTNAME.lock());

        match read_line() {
            Some(input) => parse_commands(input),
            None => {
                uefi::println!("Escape pressed. Returning to boot manager...");
                boot::stall(Duration::from_millis(800));
                return Status::SUCCESS;
            }
        }
    }
}

fn parse_commands(input: String) {
    let commands: Vec<&str> = input.split_whitespace().collect();

    if commands.is_empty() {
        return;
    }

    match commands[0] {
        "hostname" => {
            if commands.len() >= 2 {
                if commands[1] == "get" {
                    uefi::println!("Current hostname: {}", HOSTNAME.lock());
                } else if commands[1] == "set" && commands.len() >= 3 {
                    *HOSTNAME.lock() = String::from(commands[2]);
                    uefi::println!("Hostname set: {}", HOSTNAME.lock());
                } else {
                    uefi::println!("Usage: hostname get | hostname set <name>");
                }
            } else {
                uefi::println!("Usage: hostname get | hostname set <name>");
            }
        }
        "clear" => {
            if let Err(e) = clear_screen() {
                uefi::println!("clear failed: {:?}", e);
            }
        }
        "disk" => {
            if commands.len() < 2 {
                uefi::println!("Usage: disk list | disk read <index> [lba]");
                return;
            }

            match commands[1] {
                "list" => {
                    if let Err(e) = disk::list_disks() {
                        uefi::println!("disk list failed: {:?}", e);
                    }
                }
                "read" => {
                    if commands.len() < 3 {
                        uefi::println!("Usage: disk read <index> [lba]");
                        return;
                    }

                    let index = match commands[2].parse::<usize>() {
                        Ok(i) => i,
                        Err(_) => {
                            uefi::println!("invalid index: {}", commands[2]);
                            return;
                        }
                    };

                    let lba = if commands.len() >= 4 {
                        match commands[3].parse::<u64>() {
                            Ok(n) => n,
                            Err(_) => {
                                uefi::println!("invalid lba: {}", commands[3]);
                                return;
                            }
                        }
                    } else {
                        0
                    };

                    let handle = match disk::get_disk_handle(index) {
                        Ok(h) => h,
                        Err(e) => {
                            uefi::println!("disk handle failed: {:?}", e);
                            return;
                        }
                    };

                    let sector = match disk::read_sector(handle, lba) {
                        Ok(s) => s,
                        Err(e) => {
                            uefi::println!("read failed: {:?}", e);
                            return;
                        }
                    };

                    disk::print_sector(&sector);
                }
                _ => uefi::println!("Unknown disk command: {}", commands[1]),
            }
        }
        _ => uefi::println!("Unknown command: {}", commands[0]),
    }
}
