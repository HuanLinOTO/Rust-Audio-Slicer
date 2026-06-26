mod audio;
mod slicer;

use anyhow::Result;
use clap::{Parser, Subcommand};
use hound::{WavSpec, WavWriter};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

use audio::load_audio;
use slicer::{Slicer, SlicerConfig, enforce_max_duration, is_silence, merge_short_chunks};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 音频切片处理
    Slice {
        /// 输入音频文件或目录路径
        #[arg(short, long)]
        input: PathBuf,

        /// 输出目录
        #[arg(short, long)]
        output: PathBuf,

        /// 并行处理线程数 (默认为CPU核心数)
        #[arg(short, long)]
        threads: Option<usize>,

        /// 静音阈值 (dB)
        #[arg(long, default_value = "-55.0")]
        threshold_db: f32,

        /// 最小片段长度 (ms)
        #[arg(long, default_value = "1000")]
        min_length_ms: u32,

        /// 最小间隔 (ms)
        #[arg(long, default_value = "100")]
        min_interval_ms: u32,

        /// 跳跃大小 (ms)
        #[arg(long, default_value = "5")]
        hop_size_ms: u32,

        /// 最大静音长度 (ms)
        #[arg(long, default_value = "800")]
        max_silence_ms: u32,

        /// 启用切片合并
        #[arg(long, default_value = "false")]
        enable_merge: bool,

        /// 最大合并时长 (ms)
        #[arg(long, default_value = "8000")]
        max_merge_duration_ms: u32,

        /// 最大切片时长 (ms)，超过则硬切成多块；0 表示禁用
        #[arg(long, default_value = "0")]
        max_duration_ms: u32,

        /// 静音检测阈值
        #[arg(long, default_value = "0.001")]
        silence_threshold: f32,

        /// 最小有效音频占比
        #[arg(long, default_value = "0.1")]
        min_audio_ratio: f32,
    },
}

/// 性能统计结构
#[derive(Default, Clone)]
struct PerformanceStats {
    total_files: usize,
    processed_files: usize,
    total_audio_duration: f64,
    total_processing_time: f64,
    total_load_time: f64,
    total_slice_time: f64,
    total_merge_time: f64,
    total_save_time: f64,
    total_chunks_detected: usize,
    total_chunks_merged: usize,
    total_slices_saved: usize,
    total_saved_duration: f64,
}

impl PerformanceStats {
    fn add(&mut self, other: &PerformanceStats) {
        self.processed_files += other.processed_files;
        self.total_audio_duration += other.total_audio_duration;
        self.total_processing_time += other.total_processing_time;
        self.total_load_time += other.total_load_time;
        self.total_slice_time += other.total_slice_time;
        self.total_merge_time += other.total_merge_time;
        self.total_save_time += other.total_save_time;
        self.total_chunks_detected += other.total_chunks_detected;
        self.total_chunks_merged += other.total_chunks_merged;
        self.total_slices_saved += other.total_slices_saved;
        self.total_saved_duration += other.total_saved_duration;
    }
}

/// 单个文件的处理结果
struct FileProcessResult {
    file_path: PathBuf,
    stats: PerformanceStats,
    success: bool,
    error: Option<String>,
}

/// 保存音频切片
fn save_slice(samples: &[f32], sample_rate: u32, output_path: &Path) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create(output_path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

/// 计算RTF (Real Time Factor)
fn calculate_rtf(audio_duration_secs: f64, processing_time_secs: f64) -> f64 {
    if audio_duration_secs > 0.0 {
        processing_time_secs / audio_duration_secs
    } else {
        0.0
    }
}

/// 格式化时间显示
fn format_duration(duration_secs: f64) -> String {
    if duration_secs < 1.0 {
        format!("{:.1}ms", duration_secs * 1000.0)
    } else if duration_secs < 60.0 {
        format!("{duration_secs:.2}s")
    } else {
        let minutes = (duration_secs / 60.0) as u32;
        let seconds = duration_secs % 60.0;
        format!("{minutes}m{seconds:.1}s")
    }
}

/// 检查文件是否为支持的音频格式
fn is_audio_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(
            ext_str.as_str(),
            "wav" | "mp3" | "flac" | "m4a" | "aac" | "ogg"
        )
    } else {
        false
    }
}

/// 收集音频文件
fn collect_audio_files(input_path: &Path) -> Result<Vec<PathBuf>> {
    let mut audio_files = Vec::new();

    if input_path.is_file() {
        if is_audio_file(input_path) {
            audio_files.push(input_path.to_path_buf());
        } else {
            return Err(anyhow::anyhow!("输入文件不是支持的音频格式"));
        }
    } else if input_path.is_dir() {
        for entry in WalkDir::new(input_path) {
            let entry = entry?;
            if entry.file_type().is_file() && is_audio_file(entry.path()) {
                audio_files.push(entry.path().to_path_buf());
            }
        }

        if audio_files.is_empty() {
            return Err(anyhow::anyhow!("在输入目录中未找到支持的音频文件"));
        }
    } else {
        return Err(anyhow::anyhow!("输入路径不存在或无法访问"));
    }

    Ok(audio_files)
}

/// 处理配置参数结构体
#[derive(Clone)]
struct ProcessingConfig {
    config: SlicerConfig,
    silence_threshold: f32,
    min_audio_ratio: f32,
    enable_merge: bool,
    max_merge_duration_ms: u32,
    max_duration_ms: u32,
}

/// 处理单个音频文件 (线程安全版本)
#[allow(clippy::too_many_arguments)]
fn process_single_file_threaded(
    input_file: &Path,
    input_base: &Path,
    output_base: &Path,
    processing_config: &ProcessingConfig,
    progress_bar: &ProgressBar,
) -> FileProcessResult {
    let start_time = Instant::now();
    let mut result = FileProcessResult {
        file_path: input_file.to_path_buf(),
        stats: PerformanceStats::default(),
        success: false,
        error: None,
    };

    let process_result = (|| -> Result<()> {
        // 构建输出路径，保持目录结构
        let relative_path = input_file.strip_prefix(input_base)?;
        let output_dir = if let Some(parent) = relative_path.parent() {
            output_base.join(parent)
        } else {
            output_base.to_path_buf()
        };

        let file_stem = input_file.file_stem().unwrap().to_string_lossy();
        let output_file_dir = output_dir.join(&*file_stem);

        progress_bar.set_message(format!(
            "处理: {}",
            input_file.file_name().unwrap().to_string_lossy()
        ));

        // 1. 加载音频
        let load_start = Instant::now();
        let (samples, sample_rate) = load_audio(input_file)?;
        let load_duration = load_start.elapsed().as_secs_f64();
        result.stats.total_load_time += load_duration;

        let audio_duration = samples.len() as f64 / sample_rate as f64;
        result.stats.total_audio_duration += audio_duration;

        // 2. 配置切片器
        let mut slicer_cfg = processing_config.config.clone();
        slicer_cfg.sample_rate = sample_rate;
        let slicer = Slicer::new(slicer_cfg)?;

        // 3. 执行切片
        let slice_start = Instant::now();
        let mut chunks = slicer.slice(&samples);
        let slice_duration = slice_start.elapsed().as_secs_f64();
        result.stats.total_slice_time += slice_duration;
        result.stats.total_chunks_detected += chunks.len();

        // 4. 合并短片段（可选）+ 硬切超长切片
        let merge_start = Instant::now();
        if processing_config.enable_merge {
            chunks = merge_short_chunks(
                &chunks,
                processing_config.max_merge_duration_ms,
                sample_rate,
                slicer.hop_size(),
            );
        }
        if processing_config.max_duration_ms > 0 {
            chunks = enforce_max_duration(
                &chunks,
                processing_config.max_duration_ms,
                sample_rate,
                slicer.hop_size(),
            );
        }
        let merge_duration = merge_start.elapsed().as_secs_f64();
        result.stats.total_merge_time += merge_duration;
        result.stats.total_chunks_merged += chunks.len();

        // 5. 保存切片
        let save_start = Instant::now();
        std::fs::create_dir_all(&output_file_dir)?;
        let mut saved_count = 0;
        let mut file_saved_duration = 0.0;

        for &(start_frame, end_frame) in chunks.iter() {
            let start_sample = start_frame * slicer.hop_size();
            let end_sample = end_frame * slicer.hop_size();
            let slice_samples = &samples[start_sample..end_sample.min(samples.len())];

            if !is_silence(
                slice_samples,
                processing_config.silence_threshold,
                processing_config.min_audio_ratio,
            ) {
                let slice_duration = slice_samples.len() as f64 / sample_rate as f64;
                file_saved_duration += slice_duration;

                save_slice(
                    slice_samples,
                    sample_rate,
                    &output_file_dir.join(format!("slice_{saved_count:03}.wav")),
                )?;
                saved_count += 1;
            }
        }

        let save_duration = save_start.elapsed().as_secs_f64();
        result.stats.total_save_time += save_duration;
        result.stats.total_slices_saved += saved_count;
        result.stats.total_saved_duration += file_saved_duration;

        let file_processing_time = start_time.elapsed().as_secs_f64();
        result.stats.total_processing_time += file_processing_time;
        result.stats.processed_files += 1;

        progress_bar.set_message(format!(
            "完成: {} ({}个切片, RTF: {:.3}x)",
            input_file.file_name().unwrap().to_string_lossy(),
            saved_count,
            calculate_rtf(audio_duration, file_processing_time)
        ));

        Ok(())
    })();

    match process_result {
        Ok(()) => {
            result.success = true;
        }
        Err(e) => {
            result.error = Some(e.to_string());
            progress_bar.set_message(format!(
                "失败: {}",
                input_file.file_name().unwrap().to_string_lossy()
            ));
        }
    }

    progress_bar.inc(1);
    result
}

#[allow(clippy::too_many_arguments)]
fn process_slice_command(
    input: PathBuf,
    output: PathBuf,
    threads: Option<usize>,
    threshold_db: f32,
    min_length_ms: u32,
    min_interval_ms: u32,
    hop_size_ms: u32,
    max_silence_ms: u32,
    enable_merge: bool,
    max_merge_duration_ms: u32,
    max_duration_ms: u32,
    silence_threshold: f32,
    min_audio_ratio: f32,
) -> Result<()> {
    let total_start_time = Instant::now();

    // 设置线程池
    let thread_count = threads.unwrap_or_else(num_cpus::get);
    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build_global()
        .unwrap();

    println!("🎵 音频切片处理器启动");
    println!("🧵 使用 {thread_count} 个线程进行并行处理");

    // 收集音频文件
    let collect_start = Instant::now();
    let audio_files = collect_audio_files(&input)?;
    let collect_duration = collect_start.elapsed().as_secs_f64();

    println!("📂 文件扫描完成:");
    println!("   - 输入路径: {}", input.display());
    println!("   - 找到音频文件: {}个", audio_files.len());
    println!("   - 扫描用时: {}", format_duration(collect_duration));

    // 显示配置
    println!("\n⚙️  切片器配置:");
    println!("   - 静音阈值: {threshold_db}dB");
    println!("   - 最小片段长度: {min_length_ms}ms");
    println!("   - 最小间隔: {min_interval_ms}ms");
    println!("   - 跳跃大小: {hop_size_ms}ms");
    println!("   - 最大静音长度: {max_silence_ms}ms");
    println!(
        "   - 切片合并: {}",
        if enable_merge { "启用" } else { "禁用" }
    );
    if enable_merge {
        println!("   - 最大合并时长: {max_merge_duration_ms}ms");
    }
    if max_duration_ms > 0 {
        println!("   - 最大切片时长: {max_duration_ms}ms (硬切)");
    }
    println!("   - 静音检测阈值: {silence_threshold}");
    println!("   - 最小有效音频占比: {:.1}%", min_audio_ratio * 100.0);

    let config = SlicerConfig {
        sample_rate: 44100, // 临时值，会在处理时更新
        threshold_db,
        min_length_ms,
        min_interval_ms,
        hop_size_ms,
        max_silence_ms,
    };

    // 创建多进度条管理器
    let multi_progress = MultiProgress::new();
    let overall_progress = multi_progress.add(ProgressBar::new(audio_files.len() as u64));
    overall_progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("#>-")
    );

    println!("\n🔄 开始并行处理...\n");
    overall_progress.set_message("准备开始并行处理...");

    // 处理每个文件 (并行)
    let input_base = if input.is_file() {
        input.parent().unwrap_or(&input)
    } else {
        &input
    };

    let processing_start = Instant::now();
    let results: Vec<FileProcessResult> = audio_files
        .par_iter()
        .map(|audio_file| {
            process_single_file_threaded(
                audio_file,
                input_base,
                &output,
                &ProcessingConfig {
                    config: config.clone(),
                    silence_threshold,
                    min_audio_ratio,
                    enable_merge,
                    max_merge_duration_ms,
                    max_duration_ms,
                },
                &overall_progress,
            )
        })
        .collect();

    let processing_duration = processing_start.elapsed().as_secs_f64();
    overall_progress.finish_with_message("所有文件处理完成!");

    // 汇总统计结果
    let mut final_stats = PerformanceStats {
        total_files: audio_files.len(),
        ..Default::default()
    };
    let mut successful_files = 0;
    let mut failed_files = Vec::new();

    for result in results {
        if result.success {
            final_stats.add(&result.stats);
            successful_files += 1;
        } else {
            failed_files.push((
                result.file_path,
                result.error.unwrap_or_else(|| "未知错误".to_string()),
            ));
        }
    }

    // 显示失败的文件
    if !failed_files.is_empty() {
        println!("\n❌ 处理失败的文件:");
        for (file_path, error) in &failed_files {
            println!("   - {}: {}", file_path.display(), error);
        }
    }

    // 最终性能统计
    let total_duration = total_start_time.elapsed().as_secs_f64();
    let overall_rtf = calculate_rtf(
        final_stats.total_audio_duration,
        final_stats.total_processing_time,
    );

    println!("\n📊 最终性能统计:");
    println!(
        "   - 处理文件: {}/{} 个",
        successful_files, final_stats.total_files
    );
    if !failed_files.is_empty() {
        println!("   - 失败文件: {} 个", failed_files.len());
    }
    println!(
        "   - 总音频时长: {}",
        format_duration(final_stats.total_audio_duration)
    );
    println!("   - 有效切片总数: {} 个", final_stats.total_slices_saved);
    println!(
        "   - 有效音频时长: {}",
        format_duration(final_stats.total_saved_duration)
    );
    println!(
        "   - 音频保留率: {:.1}%",
        (final_stats.total_saved_duration / final_stats.total_audio_duration) * 100.0
    );

    println!("\n⏱️  各阶段用时:");
    println!("   - 文件扫描: {}", format_duration(collect_duration));
    println!(
        "   - 音频加载: {}",
        format_duration(final_stats.total_load_time)
    );
    println!(
        "   - 切片分析: {}",
        format_duration(final_stats.total_slice_time)
    );
    println!(
        "   - 片段合并: {}",
        format_duration(final_stats.total_merge_time)
    );
    println!(
        "   - 文件保存: {}",
        format_duration(final_stats.total_save_time)
    );
    println!(
        "   - 总处理时间: {}",
        format_duration(final_stats.total_processing_time)
    );
    println!(
        "   - 实际并行用时: {}",
        format_duration(processing_duration)
    );
    println!("   - 程序总用时: {}", format_duration(total_duration));

    println!("\n🚀 性能指标:");
    println!("   - 整体RTF: {overall_rtf:.3}x");
    if overall_rtf < 1.0 {
        println!("   - 处理速度比实时播放快 {:.1}倍", 1.0 / overall_rtf);
    } else {
        println!("   - 处理速度比实时播放慢 {overall_rtf:.1}倍");
    }
    if successful_files > 0 {
        println!(
            "   - 平均每个文件处理时间: {}",
            format_duration(final_stats.total_processing_time / successful_files as f64)
        );
    }

    // 计算并行加速比
    let theoretical_sequential_time = final_stats.total_processing_time;
    let speedup = theoretical_sequential_time / processing_duration;
    println!("   - 并行加速比: {speedup:.2}x (使用{thread_count}线程)");
    println!(
        "   - 并行效率: {:.1}%",
        (speedup / thread_count as f64) * 100.0
    );

    println!("\n💾 输出信息:");
    println!("   - 输出目录: {}", output.display());

    println!("\n✨ 批量处理完成！");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Slice {
            input,
            output,
            threads,
            threshold_db,
            min_length_ms,
            min_interval_ms,
            hop_size_ms,
            max_silence_ms,
            enable_merge,
            max_merge_duration_ms,
            max_duration_ms,
            silence_threshold,
            min_audio_ratio,
        } => {
            process_slice_command(
                input,
                output,
                threads,
                threshold_db,
                min_length_ms,
                min_interval_ms,
                hop_size_ms,
                max_silence_ms,
                enable_merge,
                max_merge_duration_ms,
                max_duration_ms,
                silence_threshold,
                min_audio_ratio,
            )?;
        }
    }

    Ok(())
}
