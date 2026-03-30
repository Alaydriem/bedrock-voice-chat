use anyhow::anyhow;

pub(super) struct OggOpusWriter {
    buffer: Vec<u8>,
    serial: u32,
    page_sequence: u32,
    granule_position: u64,
    sample_rate: u32,
    channels: u8,
}

impl OggOpusWriter {
    pub(super) fn new(sample_rate: u32, channels: u8) -> Result<Self, anyhow::Error> {
        let serial = rand::random::<u32>();
        let mut writer = Self {
            buffer: Vec::new(),
            serial,
            page_sequence: 0,
            granule_position: 0,
            sample_rate,
            channels,
        };

        writer.write_opus_head()?;
        writer.write_opus_tags()?;
        Ok(writer)
    }

    pub(super) fn write_packet(&mut self, data: &[u8], samples: u64) -> Result<(), anyhow::Error> {
        self.granule_position += samples;
        self.write_ogg_page(data, 0x00, self.granule_position)?;
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<Vec<u8>, anyhow::Error> {
        self.write_ogg_page(&[], 0x04, self.granule_position)?;
        Ok(self.buffer)
    }

    fn write_opus_head(&mut self) -> Result<(), anyhow::Error> {
        let mut packet = Vec::new();
        packet.extend_from_slice(b"OpusHead");
        packet.push(1);
        packet.push(self.channels);
        packet.extend_from_slice(&(0u16).to_le_bytes());
        packet.extend_from_slice(&self.sample_rate.to_le_bytes());
        packet.extend_from_slice(&(0i16).to_le_bytes());
        packet.push(0);

        self.write_ogg_page(&packet, 0x02, 0)?;
        Ok(())
    }

    fn write_opus_tags(&mut self) -> Result<(), anyhow::Error> {
        let mut packet = Vec::new();
        packet.extend_from_slice(b"OpusTags");
        let vendor = b"BVC";
        packet.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        packet.extend_from_slice(vendor);
        packet.extend_from_slice(&(0u32).to_le_bytes());

        self.write_ogg_page(&packet, 0x00, 0)?;
        Ok(())
    }

    fn write_ogg_page(
        &mut self,
        data: &[u8],
        header_type: u8,
        granule: u64,
    ) -> Result<(), anyhow::Error> {
        assert!(
            data.len() <= 255 * 255,
            "Ogg packet too large for single page"
        );
        self.buffer.extend_from_slice(b"OggS");
        self.buffer.push(0);
        self.buffer.push(header_type);
        self.buffer.extend_from_slice(&granule.to_le_bytes());
        self.buffer.extend_from_slice(&self.serial.to_le_bytes());
        self.buffer
            .extend_from_slice(&self.page_sequence.to_le_bytes());
        self.page_sequence += 1;

        let crc_offset = self.buffer.len();
        self.buffer.extend_from_slice(&[0u8; 4]);

        let segment_count = if data.is_empty() {
            0u8
        } else {
            (data.len() / 255 + 1) as u8
        };
        self.buffer.push(segment_count);

        let mut remaining = data.len();
        for _ in 0..segment_count {
            if remaining >= 255 {
                self.buffer.push(255);
                remaining -= 255;
            } else {
                self.buffer.push(remaining as u8);
                remaining = 0;
            }
        }

        self.buffer.extend_from_slice(data);

        let page_start = crc_offset - 22;
        let crc = Self::crc(&self.buffer[page_start..]);
        self.buffer[crc_offset..crc_offset + 4].copy_from_slice(&crc.to_le_bytes());

        Ok(())
    }

    fn crc(data: &[u8]) -> u32 {
        static CRC_TABLE: std::sync::LazyLock<[u32; 256]> = std::sync::LazyLock::new(|| {
            let mut table = [0u32; 256];
            for i in 0..256 {
                let mut crc = (i as u32) << 24;
                for _ in 0..8 {
                    crc = if crc & 0x80000000 != 0 {
                        (crc << 1) ^ 0x04C11DB7
                    } else {
                        crc << 1
                    };
                }
                table[i] = crc;
            }
            table
        });

        let mut crc = 0u32;
        for &byte in data {
            let index = ((crc >> 24) ^ byte as u32) as usize;
            crc = (crc << 8) ^ CRC_TABLE[index];
        }
        crc
    }
}
