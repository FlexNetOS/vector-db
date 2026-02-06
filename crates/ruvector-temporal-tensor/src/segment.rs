/// Segment binary format: encode and decode.
///
/// Format:
///   [magic:4][version:1][bits:1][group_len:4][tensor_len:4][frames:4]
///   [scale_count:4][scales:2*S][data_len:4][data:D]
///
/// Magic: 0x43545154 ("TQTC" in LE)

use crate::quantizer;

pub const MAGIC: u32 = 0x4354_5154; // "TQTC"
pub const VERSION: u8 = 1;
pub const HEADER_SIZE: usize = 26; // Fixed header before scales

/// Encode a segment from metadata, scales, and packed data.
pub fn encode(
    bits: u8,
    group_len: u32,
    tensor_len: u32,
    frame_count: u32,
    scales: &[u16],
    data: &[u8],
    out: &mut Vec<u8>,
) {
    out.clear();
    let estimated = HEADER_SIZE + scales.len() * 2 + data.len();
    out.reserve(estimated);

    // Header
    out.extend_from_slice(&MAGIC.to_le_bytes());
    out.push(VERSION);
    out.push(bits);
    out.extend_from_slice(&group_len.to_le_bytes());
    out.extend_from_slice(&tensor_len.to_le_bytes());
    out.extend_from_slice(&frame_count.to_le_bytes());

    // Scales
    let scale_count = scales.len() as u32;
    out.extend_from_slice(&scale_count.to_le_bytes());
    for &s in scales {
        out.extend_from_slice(&s.to_le_bytes());
    }

    // Data
    let data_len = data.len() as u32;
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(data);
}

/// Decoded segment header.
#[derive(Debug, Clone)]
pub struct SegmentHeader {
    pub bits: u8,
    pub group_len: u32,
    pub tensor_len: u32,
    pub frame_count: u32,
    pub scale_count: u32,
}

/// Decode a segment, returning all frames as f32 values.
pub fn decode(segment: &[u8], out: &mut Vec<f32>) {
    out.clear();
    if segment.len() < HEADER_SIZE {
        return;
    }

    let mut off = 0;

    let magic = read_u32_le(segment, &mut off);
    if magic != MAGIC {
        return;
    }

    let version = segment[off];
    off += 1;
    if version != VERSION {
        return;
    }

    let bits = segment[off];
    off += 1;

    let group_len = read_u32_le(segment, &mut off);
    let tensor_len = read_u32_le(segment, &mut off);
    let frame_count = read_u32_le(segment, &mut off);
    let scale_count = read_u32_le(segment, &mut off);

    // Read scales
    let scales_end = off + (scale_count as usize) * 2;
    if scales_end > segment.len() {
        return;
    }
    let mut scales = Vec::with_capacity(scale_count as usize);
    for _ in 0..scale_count {
        scales.push(read_u16_le(segment, &mut off));
    }

    // Read data
    if off + 4 > segment.len() {
        return;
    }
    let data_len = read_u32_le(segment, &mut off) as usize;
    if off + data_len > segment.len() {
        return;
    }
    let data = &segment[off..off + data_len];

    // Dequantize
    quantizer::dequantize(
        data,
        &scales,
        group_len as usize,
        bits,
        tensor_len as usize,
        frame_count as usize,
        out,
    );
}

/// Parse only the segment header (no data decoding).
pub fn parse_header(segment: &[u8]) -> Option<SegmentHeader> {
    if segment.len() < HEADER_SIZE {
        return None;
    }
    let mut off = 0;
    let magic = read_u32_le(segment, &mut off);
    if magic != MAGIC {
        return None;
    }
    let version = segment[off];
    off += 1;
    if version != VERSION {
        return None;
    }
    let bits = segment[off];
    off += 1;
    let group_len = read_u32_le(segment, &mut off);
    let tensor_len = read_u32_le(segment, &mut off);
    let frame_count = read_u32_le(segment, &mut off);
    let scale_count = read_u32_le(segment, &mut off);

    Some(SegmentHeader {
        bits,
        group_len,
        tensor_len,
        frame_count,
        scale_count,
    })
}

fn read_u32_le(bytes: &[u8], offset: &mut usize) -> u32 {
    let o = *offset;
    let arr = [bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]];
    *offset = o + 4;
    u32::from_le_bytes(arr)
}

fn read_u16_le(bytes: &[u8], offset: &mut usize) -> u16 {
    let o = *offset;
    let arr = [bytes[o], bytes[o + 1]];
    *offset = o + 2;
    u16::from_le_bytes(arr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantizer;

    #[test]
    fn test_encode_decode_roundtrip() {
        let frame: Vec<f32> = (0..128).map(|i| (i as f32 - 64.0) * 0.1).collect();
        let group_len = 64usize;
        let bits = 8u8;

        let scales = quantizer::compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantizer::quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut seg = Vec::new();
        encode(bits, group_len as u32, frame.len() as u32, 1, &scales, &packed, &mut seg);

        let mut decoded = Vec::new();
        decode(&seg, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        for (i, (&orig, &dec)) in frame.iter().zip(decoded.iter()).enumerate() {
            let err = (orig - dec).abs();
            assert!(err < 0.1, "i={i} orig={orig} dec={dec} err={err}");
        }
    }

    #[test]
    fn test_magic_validation() {
        let mut decoded = Vec::new();
        decode(&[0, 0, 0, 0], &mut decoded);
        assert!(decoded.is_empty()); // Wrong magic
    }

    #[test]
    fn test_parse_header() {
        let frame = vec![1.0f32; 64];
        let scales = quantizer::compute_scales(&frame, 64, 7);
        let mut packed = Vec::new();
        quantizer::quantize_and_pack(&frame, &scales, 64, 7, &mut packed);

        let mut seg = Vec::new();
        encode(7, 64, 64, 1, &scales, &packed, &mut seg);

        let header = parse_header(&seg).unwrap();
        assert_eq!(header.bits, 7);
        assert_eq!(header.group_len, 64);
        assert_eq!(header.tensor_len, 64);
        assert_eq!(header.frame_count, 1);
    }

    #[test]
    fn test_multi_frame_roundtrip() {
        let group_len = 32usize;
        let bits = 5u8;
        let tensor_len = 64;

        let frame1: Vec<f32> = (0..tensor_len).map(|i| (i as f32) * 0.1).collect();
        let frame2: Vec<f32> = (0..tensor_len).map(|i| (i as f32) * 0.09).collect();

        let scales = quantizer::compute_scales(&frame1, group_len, bits);
        let mut packed = Vec::new();
        quantizer::quantize_and_pack(&frame1, &scales, group_len, bits, &mut packed);
        quantizer::quantize_and_pack(&frame2, &scales, group_len, bits, &mut packed);

        let mut seg = Vec::new();
        encode(bits, group_len as u32, tensor_len as u32, 2, &scales, &packed, &mut seg);

        let mut decoded = Vec::new();
        decode(&seg, &mut decoded);
        assert_eq!(decoded.len(), tensor_len * 2);
    }
}
