use uefi::Identify;
use uefi::boot;
use uefi::boot::{OpenProtocolAttributes, OpenProtocolParams};
use uefi::proto::media::block::BlockIO;

pub fn list_disks() -> uefi::Result {
    let handles = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&BlockIO::GUID))?;
    for (i, handle) in handles.iter().enumerate() {
        let block_io = unsafe {
            boot::open_protocol::<BlockIO>(
                OpenProtocolParams {
                    handle: *handle,
                    agent: boot::image_handle(),
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
        };

        let block_io = match block_io {
            Ok(b) => b,
            Err(e) => {
                uefi::println!("[{}] skipped: {:?}", i, e);
                continue;
            }
        };

        let media = block_io.media();

        uefi::println!(
            "[{}]  formatted_size={:.3} GB,  size={} blocks,  block_size={},  removable={},  PARTITION={}",
            i,
            media.last_block() as f64 / 1024.0 / 1024.0 / 1024.0 * media.block_size() as f64,
            media.last_block(),
            media.block_size(),
            media.is_removable_media(),
            media.is_logical_partition()
        );
    }

    Ok(())
}

pub fn get_disk_handle(index: usize) -> uefi::Result<uefi::Handle> {
    let handles = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&BlockIO::GUID))?;

    if index >= handles.len() {
        Err(uefi::Status::NOT_FOUND.into())
    } else {
        Ok(handles[index])
    }
}

pub fn read_sector(handle: uefi::Handle, lba: u64) -> uefi::Result<[u8; 512]> {
    let block_io = boot::open_protocol_exclusive::<BlockIO>(handle)?;
    let media = block_io.media();

    let mut buffer = [0u8; 512];
    block_io.read_blocks(media.media_id(), lba, &mut buffer)?;

    Ok(buffer)
}

pub fn write_sector(handle: uefi::Handle, lba: u64, data: &[u8; 512]) -> uefi::Result {
    let mut block_io = boot::open_protocol_exclusive::<BlockIO>(handle)?;
    let media_id = block_io.media().media_id();

    block_io.write_blocks(media_id, lba, data)?;

    Ok(())
}

pub fn print_sector(sector: &[u8; 512]) {
    for row in 0..(512 / 16) {
        let offset = row * 16;
        let chunk = &sector[offset..offset + 16];

        // offset
        uefi::print!("{:04x}  ", offset);

        // hex bytes
        for byte in chunk {
            uefi::print!("{:02x} ", byte);
        }
        uefi::print!(" ");

        // ascii
        for byte in chunk {
            let c = if *byte >= 0x20 && *byte < 0x7f {
                *byte as char
            } else {
                '.'
            };
            uefi::print!("{}", c);
        }
        uefi::println!();
    }
}
