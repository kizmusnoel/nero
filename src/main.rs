#![no_main]
#![no_std]

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
    *HOSTNAME.lock() = String::from("local");

    uefi::helpers::init().unwrap();

    if let Err(e) = clear_screen() {
        uefi::println!("clear_screen failed: {:?}", e);
    }

    uefi::println!("WELCOME TO NERO (New Rust-based Operating System)!");
    uefi::println!("(Press Esc at any time to exit to the next boot option)");

    loop {
        uefi::print!("super@{} > ", HOSTNAME.lock());

        match read_line() {
            Some(input) => {
                parse_commands(input);
            }
            None => {
                uefi::println!("Escape pressed. Returning to boot manager...");
                boot::stall(Duration::from_millis(800)); // brief pause so the message is visible
                return Status::SUCCESS;
            }
        }
    }
}

fn parse_commands(input: String) {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "hostname" if parts.len() >= 2 && parts[1] == "set" && parts.len() >= 3 => {
            *HOSTNAME.lock() = String::from(parts[2]);
            uefi::println!("hostname set: {}", HOSTNAME.lock());
        }
        "hostname" if parts.len() >= 2 && parts[1] == "get" => {
            uefi::println!("hostname is: {}", HOSTNAME.lock());
        }
        _ => uefi::println!("input: {:?}", parts),
    }
}
