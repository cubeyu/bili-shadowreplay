use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::progress_event::ProgressReporterTrait;
use async_ffmpeg_sidecar::event::FfmpegEvent;
use async_ffmpeg_sidecar::log_parser::FfmpegLogParser;
use tokio::io::BufReader;

pub struct TranscodeConfig {
    pub input_path: PathBuf,
    pub input_format: String,
    pub output_path: PathBuf,
}

pub struct TranscodeResult {
    pub output_path: PathBuf,
}

pub async fn transcode(
    reporter: &impl ProgressReporterTrait,
    config: TranscodeConfig,
) -> Result<TranscodeResult, String> {
    let input_path = config.input_path;
    let input_format = config.input_format;
    let output_path = config.output_path;

    log::info!(
        "Transcode task start: input_path: {}, output_path: {}",
        input_path.display(),
        output_path.display()
    );

    log::info!(
        "FFMPEG version: {:?}",
        async_ffmpeg_sidecar::version::ffmpeg_version().await
    );

    let child = tokio::process::Command::new("ffmpeg")
        .args(["-f", input_format.as_str()])
        .args(["-i", input_path.to_str().unwrap()])
        .args(["-c", "copy"])
        .args(["-y", output_path.to_str().unwrap()])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        log::error!("Transcode error: {}", e);
        return Err(e.to_string());
    }

    let mut child = child.unwrap();

    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    let mut extract_error = None;

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("Transcode error: {}", e);
                extract_error = Some(e.to_string());
            }
            FfmpegEvent::Progress(p) => reporter.update(format!("编码中：{}", p.time).as_str()),
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    if let Some(error) = extract_error {
        log::error!("Transcode error: {}", error);
        return Err(error);
    }

    log::info!(
        "Transcode task end: output_path: {}",
        &output_path.display()
    );

    Ok(TranscodeResult { output_path })
}

pub async fn extract_audio(file: &Path) -> Result<(), String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -ar 16000 test.wav
    log::info!("Extract audio task start: {}", file.display());
    let output_path = file.with_extension("wav");
    let mut extract_error = None;

    let child = tokio::process::Command::new("ffmpeg")
        .args(["-i", file.to_str().unwrap()])
        .args(["-ar", "16000"])
        .args([output_path.to_str().unwrap()])
        .args(["-y"])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("Extract audio error: {}", e);
                extract_error = Some(e.to_string());
            }
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    if let Some(error) = extract_error {
        log::error!("Extract audio error: {}", error);
        Err(error)
    } else {
        log::info!("Extract audio task end: {}", output_path.display());
        Ok(())
    }
}

pub async fn encode_video_subtitle(
    reporter: &impl ProgressReporterTrait,
    file: &Path,
    subtitle: &Path,
    srt_style: String,
) -> Result<String, String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -vf "subtitles=test.srt:force_style='FontSize=24'" -c:v libx264 -c:a copy output.mp4
    log::info!("Encode video subtitle task start: {}", file.display());
    log::info!("srt_style: {}", srt_style);
    // output path is file with prefix [subtitle]
    let output_filename = format!("[subtitle]{}", file.file_name().unwrap().to_str().unwrap());
    let output_path = file.with_file_name(&output_filename);

    // check output path exists
    if output_path.exists() {
        log::info!("Output path already exists: {}", output_path.display());
        return Err("Output path already exists".to_string());
    }

    let mut command_error = None;

    // if windows
    let subtitle = if cfg!(target_os = "windows") {
        // escape characters in subtitle path
        let subtitle = subtitle
            .to_str()
            .unwrap()
            .replace("\\", "\\\\")
            .replace(":", "\\:");
        format!("'{}'", subtitle)
    } else {
        format!("'{}'", subtitle.display())
    };
    let vf = format!("subtitles={}:force_style='{}'", subtitle, srt_style);
    log::info!("vf: {}", vf);

    let child = tokio::process::Command::new("ffmpeg")
        .args(["-i", file.to_str().unwrap()])
        .args(["-vf", vf.as_str()])
        .args(["-c:v", "libx264"])
        .args(["-c:a", "copy"])
        .args([output_path.to_str().unwrap()])
        .args(["-y"])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("Encode video subtitle error: {}", e);
                command_error = Some(e.to_string());
            }
            FfmpegEvent::Progress(p) => {
                log::info!("Encode video subtitle progress: {}", p.time);
                reporter.update(format!("压制中：{}", p.time).as_str());
            }
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    if let Err(e) = child.wait().await {
        log::error!("Encode video subtitle error: {}", e);
        return Err(e.to_string());
    }

    if let Some(error) = command_error {
        log::error!("Encode video subtitle error: {}", error);
        Err(error)
    } else {
        log::info!("Encode video subtitle task end: {}", output_path.display());
        Ok(output_filename)
    }
}
