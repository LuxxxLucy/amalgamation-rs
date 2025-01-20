use crate::progress::ProgressTracker;
use anyhow::Result;
use reqwest::Client;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;
use walkdir::WalkDir;
use zip::ZipArchive;

pub struct AmalgamationAction {
    pub url: String,
    pub output_pathname: PathBuf,
    pub verbose: bool,
}

impl AmalgamationAction {
    fn log_progress(&self, progress: &ProgressTracker, message: &str) {
        progress.set_stage(message);
        if self.verbose {
            println!("{}", message);
        }
    }

    pub async fn execute(&self) -> Result<()> {
        let progress = ProgressTracker::new();

        // Stage 1: Resolving URL
        self.log_progress(&progress, "Resolving repository URL...");
        let resolved_url = resolve_url(&self.url);
        if self.verbose {
            println!("Resolved URL: {}", resolved_url);
        }

        // Stage 2: Downloading
        self.log_progress(&progress, "Downloading repository...");
        let zip_content = self.download_repository(&resolved_url).await?;

        // Stage 3: Extracting
        self.log_progress(&progress, "Extracting files...");
        let temp_dir = TempDir::new()?;
        self.extract_zip(&zip_content, &temp_dir)?;

        // Stage 4: Analyzing files
        self.log_progress(&progress, "Analyzing files...");
        let mut files = self.collect_all_files(&temp_dir)?;
        files.sort();

        // Stage 5: Writing to file
        self.log_progress(&progress, "Writing files...");
        write_files(&files, &self.output_pathname)?;

        // Stage 6: Success
        progress.finish();
        Ok(())
    }

    pub async fn download_repository(&self, resolved_url: &str) -> Result<Vec<u8>> {
        let client = Client::new();
        let archive_url = format!("{}/zipball/master", resolved_url);

        let response = client.get(&archive_url).send().await?;

        let content = response.bytes().await?;

        if self.verbose {
            let size_mb = content.len() as f64 / 1_048_576.0;
            println!("Downloaded repository archive:");
            println!("  URL: {}", archive_url);
            println!("  Size: {:.2} MB", size_mb);
            println!("  Target: {}", self.output_pathname.display());
        }

        Ok(content.to_vec())
    }

    pub fn extract_zip(&self, zip_content: &[u8], temp_dir: &TempDir) -> Result<()> {
        let reader = std::io::Cursor::new(zip_content);
        let mut archive = ZipArchive::new(reader)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = temp_dir.path().join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Ok(())
    }

    fn collect_all_files(&self, dir: &TempDir) -> Result<Vec<PathBuf>> {
        let mut source_files = Vec::new();

        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                source_files.push(path.to_path_buf());
            }
        }

        Ok(source_files)
    }
}

pub fn resolve_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    url.trim_end_matches(".git").to_string()
}

pub fn write_files(files: &[PathBuf], output_path: &PathBuf) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output_file = File::create(output_path)?;

    for file_path in files {
        // Skip if not a regular file
        if !file_path.is_file() {
            continue;
        }

        // Write file header
        writeln!(output_file, "// File: {}", file_path.display())?;

        // Read and write file content
        match fs::read_to_string(file_path) {
            Ok(content) => {
                writeln!(output_file, "{}\n", content)?;
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not read file {}: {}",
                    file_path.display(),
                    e
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_resolution() {
        let test_cases = vec![
            (
                "https://github.com/typst/typst.git",
                "https://github.com/typst/typst",
            ),
            (
                "https://github.com/typst/typst/",
                "https://github.com/typst/typst",
            ),
            (
                "https://github.com/typst/typst",
                "https://github.com/typst/typst",
            ),
            (
                "https://github.com/typst/typst.git/",
                "https://github.com/typst/typst",
            ),
        ];

        for (input, expected) in test_cases {
            assert_eq!(resolve_url(input), expected);
        }
    }
}
