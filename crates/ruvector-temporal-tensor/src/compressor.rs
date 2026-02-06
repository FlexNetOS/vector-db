/// TemporalTensorCompressor: the main entry point.
///
/// Manages temporal segments, drift detection, and tier transitions.

use crate::quantizer;
use crate::segment;
use crate::tier_policy::TierPolicy;

pub struct TemporalTensorCompressor {
    policy: TierPolicy,
    len: u32,

    access_count: u32,
    last_access_ts: u32,

    active_bits: u8,
    active_group_len: usize,
    active_scales: Vec<u16>,
    active_frames: u32,
    active_data: Vec<u8>,
}

impl TemporalTensorCompressor {
    /// Create a new compressor for tensors of the given length.
    pub fn new(policy: TierPolicy, len: u32, now_ts: u32) -> Self {
        let bits = policy.select_bits(0, now_ts, now_ts);
        Self {
            policy,
            len,
            access_count: 0,
            last_access_ts: now_ts,
            active_bits: bits,
            active_group_len: policy.group_len.max(1) as usize,
            active_scales: Vec::new(),
            active_frames: 0,
            active_data: Vec::new(),
        }
    }

    /// Record an access (increments count, updates timestamp).
    pub fn touch(&mut self, now_ts: u32) {
        self.access_count = self.access_count.wrapping_add(1);
        self.last_access_ts = now_ts;
    }

    /// Set access stats directly (for restoring state).
    pub fn set_access(&mut self, access_count: u32, last_access_ts: u32) {
        self.access_count = access_count;
        self.last_access_ts = last_access_ts;
    }

    /// Current tier bits.
    pub fn active_bits(&self) -> u8 {
        self.active_bits
    }

    /// Number of frames in the current segment.
    pub fn active_frame_count(&self) -> u32 {
        self.active_frames
    }

    /// Push a frame. If a segment boundary is crossed, the completed segment
    /// bytes are written to `out_segment`. Otherwise `out_segment` is cleared.
    pub fn push_frame(&mut self, frame: &[f32], now_ts: u32, out_segment: &mut Vec<u8>) {
        out_segment.clear();

        if frame.len() != self.len as usize {
            return;
        }

        let desired_bits = self.policy.select_bits(
            self.access_count,
            self.last_access_ts,
            now_ts,
        );
        let drift_factor = self.policy.drift_factor();

        let need_new_segment = self.active_frames == 0
            || desired_bits != self.active_bits
            || !quantizer::frame_fits_scales(
                frame,
                &self.active_scales,
                self.active_group_len,
                self.active_bits,
                drift_factor,
            );

        if need_new_segment {
            self.flush(out_segment);
            self.active_bits = desired_bits;
            self.active_group_len = self.policy.group_len.max(1) as usize;
            self.active_scales = quantizer::compute_scales(
                frame,
                self.active_group_len,
                self.active_bits,
            );
        }

        quantizer::quantize_and_pack(
            frame,
            &self.active_scales,
            self.active_group_len,
            self.active_bits,
            &mut self.active_data,
        );
        self.active_frames = self.active_frames.wrapping_add(1);
    }

    /// Flush the current segment. Writes segment bytes to `out_segment`.
    /// Resets internal state for the next segment.
    pub fn flush(&mut self, out_segment: &mut Vec<u8>) {
        if self.active_frames == 0 {
            return;
        }

        segment::encode(
            self.active_bits,
            self.active_group_len as u32,
            self.len,
            self.active_frames,
            &self.active_scales,
            &self.active_data,
            out_segment,
        );

        self.active_frames = 0;
        self.active_scales.clear();
        self.active_data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_policy() -> TierPolicy {
        TierPolicy::default()
    }

    #[test]
    fn test_create_and_push() {
        let mut comp = TemporalTensorCompressor::new(default_policy(), 64, 0);
        let frame = vec![1.0f32; 64];
        let mut seg = Vec::new();

        comp.push_frame(&frame, 0, &mut seg);
        assert!(seg.is_empty()); // First frame, no completed segment
        assert_eq!(comp.active_frame_count(), 1);
    }

    #[test]
    fn test_flush_produces_segment() {
        let mut comp = TemporalTensorCompressor::new(default_policy(), 64, 0);
        let frame = vec![1.0f32; 64];
        let mut seg = Vec::new();

        comp.push_frame(&frame, 0, &mut seg);
        comp.flush(&mut seg);

        assert!(!seg.is_empty());
        // Verify we can decode it
        let mut decoded = Vec::new();
        segment::decode(&seg, &mut decoded);
        assert_eq!(decoded.len(), 64);
    }

    #[test]
    fn test_tier_transition_flushes() {
        let policy = TierPolicy {
            hot_min_score: 512,
            warm_min_score: 64,
            warm_bits: 7,
            drift_pct_q8: 26,
            group_len: 64,
        };

        let mut comp = TemporalTensorCompressor::new(policy, 64, 0);
        comp.set_access(100, 0); // Hot
        let frame = vec![1.0f32; 64];
        let mut seg = Vec::new();

        comp.push_frame(&frame, 1, &mut seg);
        assert_eq!(comp.active_bits(), 8); // Hot

        // Make it cold
        comp.set_access(1, 0);
        comp.push_frame(&frame, 10000, &mut seg);
        // Should have flushed the hot segment
        assert!(!seg.is_empty());
        assert_eq!(comp.active_bits(), 3); // Cold
    }

    #[test]
    fn test_drift_triggers_new_segment() {
        let mut comp = TemporalTensorCompressor::new(default_policy(), 64, 0);
        let mut seg = Vec::new();

        let frame1 = vec![1.0f32; 64];
        comp.push_frame(&frame1, 0, &mut seg);

        // Frame with values that exceed drift threshold
        let frame2 = vec![5.0f32; 64];
        comp.push_frame(&frame2, 0, &mut seg);

        // Drift should have triggered a new segment
        assert!(!seg.is_empty());
    }

    #[test]
    fn test_multi_frame_same_segment() {
        let mut comp = TemporalTensorCompressor::new(default_policy(), 64, 0);
        let mut seg = Vec::new();

        let frame = vec![1.0f32; 64];
        comp.push_frame(&frame, 0, &mut seg);
        assert!(seg.is_empty());

        // Slightly different frame within drift
        let frame2 = vec![1.05f32; 64];
        comp.push_frame(&frame2, 0, &mut seg);
        assert!(seg.is_empty()); // Same segment, no flush
        assert_eq!(comp.active_frame_count(), 2);
    }

    #[test]
    fn test_full_roundtrip() {
        let mut comp = TemporalTensorCompressor::new(default_policy(), 128, 0);
        // Make compressor hot (8-bit) for tight accuracy test
        comp.set_access(100, 0);
        let frame: Vec<f32> = (0..128).map(|i| (i as f32 - 64.0) * 0.01).collect();
        let mut seg = Vec::new();

        // Push several similar frames
        for _ in 0..10 {
            comp.push_frame(&frame, 1, &mut seg);
        }
        comp.flush(&mut seg);

        let mut decoded = Vec::new();
        segment::decode(&seg, &mut decoded);
        assert_eq!(decoded.len(), 128 * 10);

        // Check first frame's values (8-bit: max error < 0.8%)
        let max_abs = frame.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        for i in 0..128 {
            let err = (decoded[i] - frame[i]).abs();
            assert!(err < max_abs * 0.02, "i={i} orig={} dec={} err={err}", frame[i], decoded[i]);
        }
    }

    #[test]
    fn test_wrong_length_frame_rejected() {
        let mut comp = TemporalTensorCompressor::new(default_policy(), 64, 0);
        let frame = vec![1.0f32; 32]; // Wrong length
        let mut seg = Vec::new();
        comp.push_frame(&frame, 0, &mut seg);
        assert_eq!(comp.active_frame_count(), 0);
    }
}
