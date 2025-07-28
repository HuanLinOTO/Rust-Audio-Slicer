use anyhow::Result;

/// 音频切片器配置参数
#[derive(Debug, Clone)]
pub struct SlicerConfig {
    pub sample_rate: u32,
    pub threshold_db: f32,
    pub min_length_ms: u32,
    pub min_interval_ms: u32,
    pub hop_size_ms: u32,
    pub max_silence_ms: u32,
}

/// 音频切片器
pub struct Slicer {
    hop_size: usize,
    win_size: usize,
    min_length: usize,
    min_interval: usize,
    max_silence: usize,
    threshold: f32,
}

impl Slicer {
    pub fn new(cfg: SlicerConfig) -> Result<Self> {
        // 验证参数有效性
        if cfg.min_length_ms < cfg.min_interval_ms || cfg.min_interval_ms < cfg.hop_size_ms {
            return Err(anyhow::anyhow!(
                "必须满足: min_length >= min_interval >= hop_size"
            ));
        }
        if cfg.max_silence_ms < cfg.hop_size_ms {
            return Err(anyhow::anyhow!("必须满足: max_silence >= hop_size"));
        }

        // 转换时间单位为样本帧数
        let hop_size = (cfg.sample_rate as f32 * cfg.hop_size_ms as f32 / 1000.0).round() as usize;
        let min_interval =
            (cfg.sample_rate as f32 * cfg.min_interval_ms as f32 / 1000.0).round() as usize;
        let win_size = min_interval.min(4 * hop_size);

        Ok(Self {
            hop_size,
            win_size,
            min_length: (cfg.sample_rate as f32 * cfg.min_length_ms as f32
                / 1000.0
                / hop_size as f32)
                .round() as usize,
            min_interval: (min_interval as f32 / hop_size as f32).round() as usize,
            max_silence: (cfg.sample_rate as f32 * cfg.max_silence_ms as f32
                / 1000.0
                / hop_size as f32)
                .round() as usize,
            threshold: 10f32.powf(cfg.threshold_db / 20.0), // dB转线性值
        })
    }

    pub fn hop_size(&self) -> usize {
        self.hop_size
    }

    /// 执行音频切片
    pub fn slice(&self, samples: &[f32]) -> Vec<(usize, usize)> {
        let frame_count = samples.len().div_ceil(self.hop_size);
        let mut chunks = vec![];

        // 计算RMS能量
        let rms: Vec<f32> = (0..frame_count)
            .map(|i| {
                let start = i * self.hop_size;
                let end = (start + self.win_size).min(samples.len());
                let slice = &samples[start..end];
                (slice.iter().map(|&x| x * x).sum::<f32>() / slice.len() as f32).sqrt()
            })
            .collect();

        // 检测静音段并切片
        let mut silence_start = None;
        let mut clip_start = 0;

        for (i, &rms_val) in rms.iter().enumerate() {
            if rms_val < self.threshold {
                if silence_start.is_none() {
                    silence_start = Some(i);
                }
                continue;
            }

            if let Some(sil_start) = silence_start.take() {
                if i - sil_start > self.max_silence {
                    let clip_end = sil_start + self.min_interval;
                    if clip_end - clip_start >= self.min_length {
                        chunks.push((clip_start, clip_end));
                    }
                    clip_start = clip_end;
                }
            }
        }

        // 处理剩余音频
        if frame_count - clip_start >= self.min_length {
            chunks.push((clip_start, frame_count));
        }

        chunks
    }
}

/// 合并短片段
pub fn merge_short_chunks(
    chunks: &[(usize, usize)],
    max_duration_ms: u32,
    sample_rate: u32,
    hop_size: usize,
) -> Vec<(usize, usize)> {
    if chunks.is_empty() {
        return vec![];
    }

    let max_frames = (max_duration_ms as f32 * sample_rate as f32 / 1000.0) as usize;
    let mut merged = vec![];
    let (mut current_start, mut current_end) = chunks[0];

    for &(start, end) in &chunks[1..] {
        let current_duration = (current_end - current_start) * hop_size;
        let next_duration = (end - start) * hop_size;

        if current_duration + next_duration <= max_frames {
            current_end = end;
        } else {
            merged.push((current_start, current_end));
            current_start = start;
            current_end = end;
        }
    }

    merged.push((current_start, current_end));
    merged
}

/// 检测音频切片是否主要是静音
pub fn is_silence(samples: &[f32], threshold: f32, min_audio_ratio: f32) -> bool {
    if samples.is_empty() {
        return true;
    }

    // 计算RMS能量
    let rms = (samples.iter().map(|&x| x * x).sum::<f32>() / samples.len() as f32).sqrt();

    // 如果整体RMS低于阈值，认为是静音
    if rms < threshold {
        return true;
    }

    // 检查有效音频占比
    let non_silent_samples = samples.iter().filter(|&&x| x.abs() > threshold).count();
    let audio_ratio = non_silent_samples as f32 / samples.len() as f32;

    audio_ratio < min_audio_ratio
}
