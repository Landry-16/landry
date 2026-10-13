//! File conversions, offered in Ctrl+K on a file: images (ImageMagick), PDF (Poppler, Ghostscript),
//! documents (LibreOffice), video and audio (FFmpeg). The result is written next to the source,
//! never over an existing file, by a background run that reports when it ends.

use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::Action;
use crate::util;
use std::path::{Path, PathBuf};

const IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "heic", "avif", "svg"];
const VIDEOS: &[&str] = &["mp4", "mkv", "webm", "mov", "avi", "m4v"];
const AUDIO: &[&str] = &["mp3", "wav", "flac", "ogg", "opus", "m4a", "aac"];
const DOCS: &[&str] = &["doc", "docx", "odt", "rtf", "ppt", "pptx", "odp", "xls", "xlsx", "ods"];

/// Conversion ids and titles for this file. The id is "convert:<kind>".
pub fn actions(path: &Path) -> Vec<Action> {
    let ext = extension(path);
    let mut out: Vec<(String, String)> = Vec::new();
    let mut add = |id: &str, title: String| out.push((id.to_string(), title));
    if IMAGES.contains(&ext.as_str()) {
        for (to, label) in [("png", "PNG"), ("jpg", "JPG"), ("webp", "WebP")] {
            if !(ext == to || (ext == "jpeg" && to == "jpg")) {
                add(&format!("image-{to}"), tf("Convertir en {f}", &[("f", &label)]));
            }
        }
        add("image-half", t("Réduire de moitié"));
    } else if ext == "pdf" {
        add("pdf-png", t("Pages en images PNG"));
        add("pdf-small", t("Alléger le PDF"));
    } else if DOCS.contains(&ext.as_str()) {
        add("doc-pdf", tf("Convertir en {f}", &[("f", &"PDF")]));
    } else if VIDEOS.contains(&ext.as_str()) {
        if ext != "mp4" {
            add("video-mp4", tf("Convertir en {f}", &[("f", &"MP4")]));
        }
        if ext != "webm" {
            add("video-webm", tf("Convertir en {f}", &[("f", &"WebM")]));
        }
        add("video-gif", t("GIF des 15 premières secondes"));
        add("video-mp3", t("Extraire le son en MP3"));
    } else if AUDIO.contains(&ext.as_str()) {
        for (to, label) in [("mp3", "MP3"), ("ogg", "OGG"), ("wav", "WAV")] {
            if ext != to {
                add(&format!("audio-{to}"), tf("Convertir en {f}", &[("f", &label)]));
            }
        }
    }
    out.into_iter().map(|(id, title)| Action::new(&format!("convert:{id}"), &title)).collect()
}

fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// `dir/name.ext`, or `dir/name (2).ext` and so on when it already exists.
pub fn free_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut p = dir.join(format!("{stem}.{ext}"));
    let mut i = 2;
    while p.exists() {
        p = dir.join(format!("{stem} ({i}).{ext}"));
        i += 1;
    }
    p
}

/// ImageMagick 7 is `magick`, ImageMagick 6 (Ubuntu 24.04) is `convert`.
fn imagemagick() -> &'static str {
    if util::has("magick") { "magick" } else { "convert" }
}

/// Program, arguments and output file for a conversion.
pub fn command(path: &Path, kind: &str) -> Result<(String, Vec<String>, PathBuf), String> {
    let dir = path.parent().ok_or_else(|| t("chemin invalide"))?;
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let src = path.to_string_lossy().into_owned();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let ff = |out: &Path, opts: &[&str]| {
        let mut a = s(&["-hide_banner", "-loglevel", "error", "-y", "-i", &src]);
        a.extend(s(opts));
        a.push(out.to_string_lossy().into_owned());
        ("ffmpeg".to_string(), a)
    };
    let (program, args, out) = match kind {
        "image-png" | "image-jpg" | "image-webp" => {
            let out = free_path(dir, &stem, &kind[6..]);
            // [0]: the first frame of an animation or the first page of a document.
            (imagemagick().to_string(), vec![format!("{src}[0]"), out.to_string_lossy().into_owned()], out)
        }
        "image-half" => {
            let out = free_path(dir, &format!("{stem}-50"), &extension(path));
            (imagemagick().to_string(), vec![src.clone(), "-resize".into(), "50%".into(), out.to_string_lossy().into_owned()], out)
        }
        "pdf-png" => {
            // pdftoppm adds the page number: name-1.png, name-2.png...
            let prefix = free_path(dir, &format!("{stem}-page"), "png").with_extension("");
            ("pdftoppm".to_string(), s(&["-png", "-r", "150", &src, &prefix.to_string_lossy()]), dir.to_path_buf())
        }
        "pdf-small" => {
            let out = free_path(dir, &format!("{stem}-leger"), "pdf");
            ("gs".to_string(), s(&["-q", "-dNOPAUSE", "-dBATCH", "-sDEVICE=pdfwrite", "-dPDFSETTINGS=/ebook", &format!("-sOutputFile={}", out.display()), &src]), out)
        }
        "doc-pdf" => {
            // LibreOffice names the result itself (name.pdf) and would replace an existing one.
            let out = dir.join(format!("{stem}.pdf"));
            if out.exists() {
                return Err(tf("{p} existe déjà", &[("p", &out.display())]));
            }
            let program = if util::has("soffice") { "soffice" } else { "libreoffice" };
            (program.to_string(), s(&["--headless", "--convert-to", "pdf", "--outdir", &dir.to_string_lossy(), &src]), out)
        }
        "video-mp4" => {
            let out = free_path(dir, &stem, "mp4");
            let (p, a) = ff(&out, &["-c:v", "libx264", "-crf", "23", "-preset", "medium", "-c:a", "aac", "-movflags", "+faststart"]);
            (p, a, out)
        }
        "video-webm" => {
            let out = free_path(dir, &stem, "webm");
            let (p, a) = ff(&out, &["-c:v", "libvpx-vp9", "-crf", "33", "-b:v", "0", "-c:a", "libopus"]);
            (p, a, out)
        }
        "video-gif" => {
            let out = free_path(dir, &stem, "gif");
            let (p, a) = ff(&out, &["-t", "15", "-vf", "fps=12,scale=640:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse"]);
            (p, a, out)
        }
        "video-mp3" | "audio-mp3" => {
            let out = free_path(dir, &stem, "mp3");
            let (p, a) = ff(&out, &["-vn", "-c:a", "libmp3lame", "-q:a", "2"]);
            (p, a, out)
        }
        "audio-ogg" => {
            let out = free_path(dir, &stem, "ogg");
            let (p, a) = ff(&out, &["-vn", "-c:a", "libvorbis", "-q:a", "5"]);
            (p, a, out)
        }
        "audio-wav" => {
            let out = free_path(dir, &stem, "wav");
            let (p, a) = ff(&out, &["-vn"]);
            (p, a, out)
        }
        k => return Err(tf("action inconnue : {a}", &[("a", &k)])),
    };
    Ok((program, args, out))
}

/// Package that provides each conversion program, for the error message.
fn package(program: &str) -> &'static str {
    match program {
        "ffmpeg" => "ffmpeg",
        "pdftoppm" => "poppler-utils",
        "gs" => "ghostscript",
        "soffice" | "libreoffice" => "libreoffice",
        _ => "imagemagick",
    }
}

pub fn run(path: &Path, kind: &str) -> Result<Outcome, String> {
    let (program, args, out) = command(path, kind)?;
    if !util::has(&program) {
        return Err(tf("{p} manque pour cette conversion : sudo apt install {pkg}", &[("p", &program), ("pkg", &package(&program))]));
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let target = if out.is_dir() { t("images dans le même dossier") } else { out.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() };
    Ok(Outcome::Run { program, args, label: tf("{src} vers {dst}", &[("src", &name), ("dst", &target)]), silent: false, limit: 3600 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(p: &str) -> Vec<String> {
        actions(Path::new(p)).into_iter().map(|a| a.id).collect()
    }

    #[test]
    fn offers_conversions_by_type() {
        assert_eq!(ids("/x/photo.png"), ["convert:image-jpg", "convert:image-webp", "convert:image-half"]);
        assert_eq!(ids("/x/scan.PDF"), ["convert:pdf-png", "convert:pdf-small"]);
        assert!(ids("/x/clip.mp4").contains(&"convert:video-gif".to_string()));
        assert!(!ids("/x/clip.mp4").contains(&"convert:video-mp4".to_string()));
        assert_eq!(ids("/x/rapport.docx"), ["convert:doc-pdf"]);
        assert!(ids("/x/notes.txt").is_empty());
    }

    #[test]
    fn never_overwrites() {
        let dir = std::env::temp_dir().join(format!("landry-convert-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), "").unwrap();
        assert_eq!(free_path(&dir, "a", "jpg"), dir.join("a (2).jpg"));
        let (_, args, out) = command(&dir.join("a.png"), "image-jpg").unwrap();
        assert_eq!(out, dir.join("a (2).jpg"));
        assert!(args[0].ends_with("a.png[0]"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
