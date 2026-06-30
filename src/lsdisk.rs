extern crate alloc;
use uefi::boot;
use uefi::proto::media::block::BlockIO;
use uefi::boot::{OpenProtocolAttributes, OpenProtocolParams};
use uefi::Identify;


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
            "[{}] size={} blocks  block_size={}  removable={} partition={} media_id={}",
            i,
            media.last_block(),
            media.block_size(),
            media.is_removable_media(),
            media.is_logical_partition(),
            media.media_id()
        );
    }

    Ok(())
}

pub fn get_disk_handle(index: usize) -> uefi::Result<uefi::Handle> {
    let handles = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&BlockIO::GUID))?;
    Ok(handles[index])
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