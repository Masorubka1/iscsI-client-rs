use anyhow::{Context, Result, bail};

use crate::{
    client::pdu_connection::FromBytes,
    models::{
        common::BasicHeaderSegment, opcode::BhsOpcode,
        reject::reject_description::RejectReason,
    },
};

/// BHS for a Reject PDU (always 52 bytes)
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectPdu {
    pub opcode: BhsOpcode, // always 0x3f
    reserved1: u8,
    pub reason: RejectReason,
    pub reserved2: u8,
    pub total_ahs_length: u8,
    pub data_segment_length: [u8; 3],
    pub reserved3: [u8; 4 * 2],
    pub itt: u32, // always 0xffffffff
    pub reserved4: [u8; 4],
    pub stat_sn: u32,
    pub exp_cmd_sn: u32,
    pub max_cmd_sn: u32,
    pub data_sn_or_r2_sn: u32,
    pub reserved5: [u8; 4 * 2],
    pub header_diggest: u32,
}

impl RejectPdu {
    pub const HEADER_LEN: usize = 52;

    pub fn from_bhs_bytes(buf: &[u8; Self::HEADER_LEN]) -> Result<Self> {
        let opcode = BhsOpcode::try_from(buf[0])?;
        let reserved1 = buf[1];
        let reason = RejectReason::try_from(buf[2])?;
        let reserved2 = buf[3];
        let total_ahs_length = buf[4];
        let data_segment_length = [buf[5], buf[6], buf[7]];
        let mut reserved3 = [0u8; 8];
        reserved3.copy_from_slice(&buf[8..16]);
        let itt = u32::from_be_bytes(buf[16..20].try_into()?);
        let mut reserved4 = [0u8; 4];
        reserved4.copy_from_slice(&buf[20..24]);
        let stat_sn = u32::from_be_bytes(buf[24..28].try_into()?);
        let exp_cmd_sn = u32::from_be_bytes(buf[28..32].try_into()?);
        let max_cmd_sn = u32::from_be_bytes(buf[32..36].try_into()?);
        let data_sn_or_r2_sn = u32::from_be_bytes(buf[36..40].try_into()?);
        let mut reserved5 = [0u8; 4 * 2];
        reserved5.copy_from_slice(&buf[40..48]);
        let header_diggest = u32::from_be_bytes(buf[48..52].try_into()?);

        Ok(RejectPdu {
            opcode,
            reserved1,
            reason,
            reserved2,
            total_ahs_length,
            data_segment_length,
            reserved3,
            itt,
            reserved4,
            stat_sn,
            exp_cmd_sn,
            max_cmd_sn,
            data_sn_or_r2_sn,
            reserved5,
            header_diggest,
        })
    }

    pub fn to_bhs_bytes(&self) -> [u8; Self::HEADER_LEN] {
        let mut buf = [0u8; Self::HEADER_LEN];
        buf[0] = self.opcode.clone().into();
        buf[1] = self.reserved1;
        buf[2] = self.reason.into();
        buf[3] = self.reserved2;
        buf[4] = self.total_ahs_length;
        buf[5..8].copy_from_slice(&self.data_segment_length);
        buf[8..16].copy_from_slice(&self.reserved3);
        buf[16..20].copy_from_slice(&self.itt.to_be_bytes());
        buf[20..24].copy_from_slice(&self.reserved4);
        buf[24..28].copy_from_slice(&self.stat_sn.to_be_bytes());
        buf[28..32].copy_from_slice(&self.exp_cmd_sn.to_be_bytes());
        buf[32..36].copy_from_slice(&self.max_cmd_sn.to_be_bytes());
        buf[36..40].copy_from_slice(&self.data_sn_or_r2_sn.to_be_bytes());
        buf[40..48].copy_from_slice(&self.reserved5);
        buf[48..52].copy_from_slice(&self.header_diggest.to_be_bytes());
        buf
    }

    /// Parsing PDU with DataSegment and Digest
    pub fn parse(buf: &[u8]) -> Result<(Self, Vec<u8>, Option<u32>)> {
        if buf.len() < Self::HEADER_LEN {
            bail!(
                "Buffer {} too small for NopInResponse BHS {}",
                buf.len(),
                Self::HEADER_LEN
            );
        }

        let mut bhs = [0u8; Self::HEADER_LEN];
        bhs.copy_from_slice(&buf[..Self::HEADER_LEN]);
        let header = Self::from_bhs_bytes(&bhs)?;

        let ahs_len = header.ahs_length_bytes();
        let data_len = header.data_length_bytes();
        let mut offset = Self::HEADER_LEN + ahs_len;

        if buf.len() < offset + data_len {
            bail!(
                "RejectPdu Buffer {} too small for DataSegment {}",
                buf.len(),
                offset + data_len
            );
        }
        let data = buf[offset..offset + data_len].to_vec();
        offset += data_len;

        let hd = if buf.len() >= offset + 4 {
            Some(u32::from_be_bytes(
                buf[offset..offset + 4]
                    .try_into()
                    .context("Failed to get offset from buf")?,
            ))
        } else {
            None
        };

        Ok((header, data, hd))
    }
}

impl BasicHeaderSegment for RejectPdu {
    fn get_opcode(&self) -> BhsOpcode {
        self.opcode.clone()
    }

    fn ahs_length_bytes(&self) -> usize {
        (self.total_ahs_length as usize) * 4
    }

    fn data_length_bytes(&self) -> usize {
        let data_size = u32::from_be_bytes([
            0,
            self.data_segment_length[0],
            self.data_segment_length[1],
            self.data_segment_length[2],
        ]) as usize;

        let pad = (4 - (data_size % 4)) % 4;
        data_size + pad
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bhs_bytes().to_vec()
    }

    fn from_bytes(buf: &[u8]) -> Result<Self> {
        let mut new_buf = [0u8; RejectPdu::HEADER_LEN];
        new_buf.clone_from_slice(buf);
        RejectPdu::from_bhs_bytes(&new_buf)
    }
}

impl FromBytes for RejectPdu {
    const HEADER_LEN: usize = Self::HEADER_LEN;

    fn peek_total_len(buf: &[u8]) -> Result<usize> {
        if buf.len() < Self::HEADER_LEN {
            bail!(
                "Buffer {} too small for RejectPdu BHS {}",
                buf.len(),
                Self::HEADER_LEN
            );
        }

        let mut b = [0u8; Self::HEADER_LEN];
        b.copy_from_slice(&buf[..Self::HEADER_LEN]);
        let hdr = RejectPdu::from_bhs_bytes(&b)?;

        let ahs_len = hdr.ahs_length_bytes();
        let data_len = hdr.data_length_bytes();

        Ok(Self::HEADER_LEN + ahs_len + data_len)
    }

    fn from_bytes(buf: &[u8]) -> Result<(Self, Vec<u8>, Option<u32>)> {
        let mut hdr = [0u8; Self::HEADER_LEN];
        hdr.copy_from_slice(&buf[..Self::HEADER_LEN]);
        let pdu = RejectPdu::from_bhs_bytes(&hdr)?;
        Ok((pdu, Vec::new(), None))
    }
}
