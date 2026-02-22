/// Resample multi-channel audio to 16kHz mono f32.
///
/// Uses linear interpolation for simplicity. Adequate for speech-to-text
/// where ultra-high-fidelity is not critical.
pub fn resample_to_16khz_mono(input: &[f32], input_sample_rate: u32, input_channels: u16) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    let channels = input_channels as usize;

    // Step 1: Mix down to mono by averaging channels
    let mono: Vec<f32> = input
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();

    // Step 2: Resample to 16kHz using linear interpolation
    if input_sample_rate == 16000 {
        return mono;
    }

    let ratio = 16000.0 / input_sample_rate as f64;
    let output_len = (mono.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos as usize;
        let frac = (src_pos - src_idx as f64) as f32;

        if src_idx + 1 < mono.len() {
            let sample = mono[src_idx] * (1.0 - frac) + mono[src_idx + 1] * frac;
            output.push(sample);
        } else if src_idx < mono.len() {
            output.push(mono[src_idx]);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mono_passthrough_at_16khz() {
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let result = resample_to_16khz_mono(&input, 16000, 1);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_stereo_to_mono() {
        // Stereo: L=1.0, R=0.0 should average to 0.5
        let input = vec![1.0, 0.0, 1.0, 0.0];
        let result = resample_to_16khz_mono(&input, 16000, 2);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_downsample() {
        // 48kHz to 16kHz should produce ~1/3 the samples
        let input: Vec<f32> = (0..48000).map(|i| (i as f32 / 48000.0).sin()).collect();
        let result = resample_to_16khz_mono(&input, 48000, 1);
        let expected_len = 16000;
        assert!((result.len() as i64 - expected_len as i64).unsigned_abs() < 2);
    }

    #[test]
    fn test_empty_input() {
        let result = resample_to_16khz_mono(&[], 44100, 2);
        assert!(result.is_empty());
    }
}
