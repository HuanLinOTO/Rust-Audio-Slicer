use anyhow::Result;
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// 读取音频文件并解码
pub fn load_audio<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32)> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("wav");

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;
    let track = format.default_track().unwrap();
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let sample_rate = track.codec_params.sample_rate.unwrap();
    let mut samples = Vec::new();

    while let Ok(packet) = format.next_packet() {
        let buffer = decoder.decode(&packet)?;
        match buffer {
            AudioBufferRef::F32(buf) => {
                process_f32_buffer(&buf, &mut samples);
            }
            AudioBufferRef::U8(buf) => {
                process_u8_buffer(&buf, &mut samples);
            }
            AudioBufferRef::U16(buf) => {
                process_u16_buffer(&buf, &mut samples);
            }
            AudioBufferRef::U24(buf) => {
                process_u24_buffer(&buf, &mut samples);
            }
            AudioBufferRef::U32(buf) => {
                process_u32_buffer(&buf, &mut samples);
            }
            AudioBufferRef::S8(buf) => {
                process_s8_buffer(&buf, &mut samples);
            }
            AudioBufferRef::S16(buf) => {
                process_s16_buffer(&buf, &mut samples);
            }
            AudioBufferRef::S24(buf) => {
                process_s24_buffer(&buf, &mut samples);
            }
            AudioBufferRef::S32(buf) => {
                process_s32_buffer(&buf, &mut samples);
            }
            AudioBufferRef::F64(buf) => {
                process_f64_buffer(&buf, &mut samples);
            }
        }
    }

    Ok((samples, sample_rate))
}

fn process_f32_buffer(buf: &symphonia::core::audio::AudioBuffer<f32>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += buf.chan(c)[i];
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        samples.extend_from_slice(buf.chan(0));
    }
}

fn process_u8_buffer(buf: &symphonia::core::audio::AudioBuffer<u8>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += (buf.chan(c)[i] as f32 - 128.0) / 128.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push((sample as f32 - 128.0) / 128.0);
        }
    }
}

fn process_u16_buffer(buf: &symphonia::core::audio::AudioBuffer<u16>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += (buf.chan(c)[i] as f32 - 32768.0) / 32768.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push((sample as f32 - 32768.0) / 32768.0);
        }
    }
}

fn process_u24_buffer(
    buf: &symphonia::core::audio::AudioBuffer<symphonia::core::sample::u24>,
    samples: &mut Vec<f32>,
) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                let sample_value = buf.chan(c)[i].inner() as f32;
                sum += (sample_value - 8388608.0) / 8388608.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            let sample_value = sample.inner() as f32;
            samples.push((sample_value - 8388608.0) / 8388608.0);
        }
    }
}

fn process_u32_buffer(buf: &symphonia::core::audio::AudioBuffer<u32>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += (buf.chan(c)[i] as f32 - 2147483648.0) / 2147483648.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push((sample as f32 - 2147483648.0) / 2147483648.0);
        }
    }
}

fn process_s8_buffer(buf: &symphonia::core::audio::AudioBuffer<i8>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += buf.chan(c)[i] as f32 / 128.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push(sample as f32 / 128.0);
        }
    }
}

fn process_s16_buffer(buf: &symphonia::core::audio::AudioBuffer<i16>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += buf.chan(c)[i] as f32 / 32768.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push(sample as f32 / 32768.0);
        }
    }
}

fn process_s24_buffer(
    buf: &symphonia::core::audio::AudioBuffer<symphonia::core::sample::i24>,
    samples: &mut Vec<f32>,
) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                let sample_value = buf.chan(c)[i].inner() as f32;
                sum += sample_value / 8388608.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            let sample_value = sample.inner() as f32;
            samples.push(sample_value / 8388608.0);
        }
    }
}

fn process_s32_buffer(buf: &symphonia::core::audio::AudioBuffer<i32>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += buf.chan(c)[i] as f32 / 2147483648.0;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push(sample as f32 / 2147483648.0);
        }
    }
}

fn process_f64_buffer(buf: &symphonia::core::audio::AudioBuffer<f64>, samples: &mut Vec<f32>) {
    if buf.spec().channels.count() > 1 {
        for i in 0..buf.frames() {
            let mut sum = 0.0;
            for c in 0..buf.spec().channels.count() {
                sum += buf.chan(c)[i] as f32;
            }
            samples.push(sum / buf.spec().channels.count() as f32);
        }
    } else {
        for &sample in buf.chan(0) {
            samples.push(sample as f32);
        }
    }
}
