/// Groupwise symmetric quantization with f16 scales.
///
/// For each group of `group_len` values:
///   scale = max(|v_i|) / qmax
///   q_i = round(v_i / scale), clamped to [-qmax, +qmax]
///   u_i = q_i + qmax  (bias to unsigned for packing)

use crate::bitpack::qmax_from_bits;
use crate::f16;

/// Compute f16 group scales for a frame.
pub fn compute_scales(frame: &[f32], group_len: usize, bits: u8) -> Vec<u16> {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 {
        return Vec::new();
    }
    let qmax_f = qmax as f32;

    let mut scales = Vec::with_capacity((frame.len() + group_len - 1) / group_len);
    let mut idx = 0;

    while idx < frame.len() {
        let end = (idx + group_len).min(frame.len());
        let mut max_abs = 0.0f32;

        for &v in &frame[idx..end] {
            if v.is_finite() {
                let a = v.abs();
                if a > max_abs {
                    max_abs = a;
                }
            }
        }

        let scale = if max_abs == 0.0 { 0.0 } else { max_abs / qmax_f };
        scales.push(f16::f32_to_f16_bits(scale));
        idx = end;
    }

    scales
}

/// Check if a frame fits within existing scales (within drift tolerance).
pub fn frame_fits_scales(
    frame: &[f32],
    scales: &[u16],
    group_len: usize,
    bits: u8,
    drift_factor: f32,
) -> bool {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 || scales.is_empty() {
        return false;
    }
    let qmax_f = qmax as f32;

    let mut group_idx = 0;
    let mut idx = 0;

    while idx < frame.len() {
        if group_idx >= scales.len() {
            return false;
        }

        let scale = f16::f16_bits_to_f32(scales[group_idx]);
        let allowed = scale * qmax_f * drift_factor;

        let end = (idx + group_len).min(frame.len());
        for &v in &frame[idx..end] {
            if v.is_finite() && v.abs() > allowed {
                return false;
            }
        }

        group_idx += 1;
        idx = end;
    }

    true
}

/// Quantize a frame using pre-computed scales and pack into bitstream.
pub fn quantize_and_pack(
    frame: &[f32],
    scales: &[u16],
    group_len: usize,
    bits: u8,
    out: &mut Vec<u8>,
) {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 {
        return;
    }
    let qmax_i = qmax;
    let bias = qmax;
    let bits_u32 = bits as u32;

    let mut acc: u64 = 0;
    let mut acc_bits: u32 = 0;

    let mut group_idx = 0;
    let mut idx = 0;

    while idx < frame.len() {
        let end = (idx + group_len).min(frame.len());
        let scale = f16::f16_bits_to_f32(scales[group_idx]);
        let inv_scale = if scale == 0.0 { 0.0 } else { 1.0 / scale };

        for &v in &frame[idx..end] {
            let mut q: i32 = 0;
            if v.is_finite() {
                let scaled = v * inv_scale;
                q = scaled.round() as i32;
                q = q.clamp(-qmax_i, qmax_i);
            }

            let u = (q + bias) as u32;
            acc |= (u as u64) << acc_bits;
            acc_bits += bits_u32;

            while acc_bits >= 8 {
                out.push((acc & 0xFF) as u8);
                acc >>= 8;
                acc_bits -= 8;
            }
        }

        group_idx += 1;
        idx = end;
    }

    if acc_bits > 0 {
        out.push((acc & 0xFF) as u8);
    }
}

/// Dequantize packed codes using scales, writing f32 values.
pub fn dequantize(
    data: &[u8],
    scales: &[u16],
    group_len: usize,
    bits: u8,
    tensor_len: usize,
    frame_count: usize,
    out: &mut Vec<f32>,
) {
    let qmax = qmax_from_bits(bits);
    if qmax == 0 {
        return;
    }
    let bias = qmax;
    let bits_u32 = bits as u32;
    let mask = (1u64 << bits_u32) - 1;

    let total = tensor_len * frame_count;
    out.resize(total, 0.0);

    let mut acc: u64 = 0;
    let mut acc_bits: u32 = 0;
    let mut byte_idx = 0;
    let mut val_idx = 0;

    while val_idx < total {
        // Fill accumulator
        while acc_bits < bits_u32 && byte_idx < data.len() {
            acc |= (data[byte_idx] as u64) << acc_bits;
            acc_bits += 8;
            byte_idx += 1;
        }
        if acc_bits < bits_u32 {
            break;
        }

        let u = (acc & mask) as u32;
        acc >>= bits_u32;
        acc_bits -= bits_u32;

        let q = (u as i32) - bias;
        let within_frame = val_idx % tensor_len;
        let group_idx = within_frame / group_len;

        let scale = if group_idx < scales.len() {
            f16::f16_bits_to_f32(scales[group_idx])
        } else {
            0.0
        };

        out[val_idx] = (q as f32) * scale;
        val_idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_roundtrip_8bit() {
        let frame: Vec<f32> = (0..128).map(|i| (i as f32 - 64.0) * 0.1).collect();
        let group_len = 64;
        let bits = 8;

        let scales = compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, frame.len(), 1, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        for (i, (&orig, &dec)) in frame.iter().zip(decoded.iter()).enumerate() {
            let err = (orig - dec).abs();
            let max_err = if orig.abs() > 0.01 { orig.abs() * 0.02 } else { 0.1 };
            assert!(err < max_err, "i={i}, orig={orig}, dec={dec}, err={err}");
        }
    }

    #[test]
    fn test_quantize_roundtrip_3bit() {
        let frame: Vec<f32> = (0..64).map(|i| (i as f32 - 32.0) * 0.5).collect();
        let group_len = 64;
        let bits = 3;

        let scales = compute_scales(&frame, group_len, bits);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, group_len, bits, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, group_len, bits, frame.len(), 1, &mut decoded);

        assert_eq!(decoded.len(), frame.len());
        // 3-bit has higher error but should be bounded
        let max_val = frame.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        for (&orig, &dec) in frame.iter().zip(decoded.iter()) {
            let err = (orig - dec).abs();
            assert!(err < max_val * 0.35, "orig={orig}, dec={dec}, err={err}");
        }
    }

    #[test]
    fn test_drift_detection() {
        let frame1: Vec<f32> = vec![1.0; 64];
        let frame2: Vec<f32> = vec![1.05; 64]; // 5% drift
        let frame3: Vec<f32> = vec![2.0; 64]; // 100% drift

        let scales = compute_scales(&frame1, 64, 8);
        let drift_factor = 1.0 + 26.0 / 256.0; // ~10%

        assert!(frame_fits_scales(&frame2, &scales, 64, 8, drift_factor));
        assert!(!frame_fits_scales(&frame3, &scales, 64, 8, drift_factor));
    }

    #[test]
    fn test_zero_frame() {
        let frame = vec![0.0f32; 128];
        let scales = compute_scales(&frame, 64, 8);
        let mut packed = Vec::new();
        quantize_and_pack(&frame, &scales, 64, 8, &mut packed);

        let mut decoded = Vec::new();
        dequantize(&packed, &scales, 64, 8, 128, 1, &mut decoded);

        for &v in &decoded {
            assert_eq!(v, 0.0);
        }
    }
}
