//! Image processing pipeline - Rust port of Python Click's imagepipe.py example.
//!
//! This script processes images through a simulated pipeline in a unix-pipe style.
//! One command feeds into the next using chain mode.
//!
//! Run with: cargo run --example imagepipe -- <commands...>
//!
//! Example:
//!     imagepipe open -i example01.jpg resize -w 128 display
//!     imagepipe open -i example02.jpg blur save
//!
//! Note: This is a simulation that doesn't require actual image libraries.
//! It demonstrates click-rs's chain mode command pattern.

use std::sync::{Arc, Mutex};

use click::{echo, ClickOption, Command, CommandLike, Group};

/// Simulated image with metadata
#[derive(Clone, Debug)]
struct SimulatedImage {
    filename: String,
    width: u32,
    height: u32,
    #[allow(dead_code)]
    format: String,
}

impl SimulatedImage {
    fn new(filename: &str) -> Self {
        // Simulate loading an image with default dimensions
        Self {
            filename: filename.to_string(),
            width: 1920,
            height: 1080,
            format: "JPEG".to_string(),
        }
    }
}

/// Image stream shared between chained commands
type ImageStream = Arc<Mutex<Vec<SimulatedImage>>>;

fn main() {
    // Create a shared image stream for the pipeline
    let stream: ImageStream = Arc::new(Mutex::new(Vec::new()));

    let cli = build_cli(stream.clone());

    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = CommandLike::main(&cli, args) {
        let code = e.exit_code();
        if code == 0 {
            // Help was requested - print help manually for groups
            let ctx = click::ContextBuilder::new().info_name("imagepipe").build();
            println!("{}", cli.get_help(&ctx));
        } else {
            eprintln!("{}", e.format_full());
        }
        std::process::exit(code);
    }
}

fn build_cli(stream: ImageStream) -> Group {
    let stream_open = stream.clone();
    let stream_save = stream.clone();
    let stream_display = stream.clone();
    let stream_resize = stream.clone();
    let stream_crop = stream.clone();
    let stream_blur = stream.clone();
    let stream_sharpen = stream.clone();
    let stream_emboss = stream.clone();

    Group::new("imagepipe")
        .help(
            "This script processes images through a simulated pipeline.\n\n\
             One command feeds into the next (chain mode).\n\n\
             Example:\n\n    \
             imagepipe open -i example01.jpg resize -w 128 display\n    \
             imagepipe open -i example02.jpg blur save",
        )
        .chain(true)
        .command(open_cmd(stream_open))
        .command(save_cmd(stream_save))
        .command(display_cmd(stream_display))
        .command(resize_cmd(stream_resize))
        .command(crop_cmd(stream_crop))
        .command(blur_cmd(stream_blur))
        .command(sharpen_cmd(stream_sharpen))
        .command(emboss_cmd(stream_emboss))
        .build()
}

/// Loads one or multiple images for processing.
fn open_cmd(stream: ImageStream) -> Command {
    Command::new("open")
        .help("Loads one or multiple images for processing. The input parameter can be specified multiple times.")
        .option(
            ClickOption::new(&["-i", "--image"])
                .multiple()
                .help("The image file to open.")
                .build(),
        )
        .callback(move |ctx| {
            let images = ctx.get_param::<Vec<String>>("image");

            if let Some(image_paths) = images {
                let mut stream_guard = stream.lock().unwrap();
                for path in image_paths {
                    if path == "-" {
                        echo("Opening '<stdin>'", true, false, None);
                        let img = SimulatedImage {
                            filename: "-".to_string(),
                            width: 800,
                            height: 600,
                            format: "PNG".to_string(),
                        };
                        stream_guard.push(img);
                    } else {
                        echo(&format!("Opening '{}'", path), true, false, None);
                        stream_guard.push(SimulatedImage::new(path));
                    }
                }
            }
            Ok(())
        })
        .build()
}

/// Saves all processed images to a series of files.
fn save_cmd(stream: ImageStream) -> Command {
    Command::new("save")
        .help("Saves all processed images to a series of files.")
        .option(
            ClickOption::new(&["--filename"])
                .default("processed-{:04}.png")
                .help("The format for the filename.")
                .build(),
        )
        .callback(move |ctx| {
            let filename_pattern = ctx
                .get_param::<String>("filename")
                .map(|s| s.as_str())
                .unwrap_or("processed-{:04}.png");

            let stream_guard = stream.lock().unwrap();
            for (idx, image) in stream_guard.iter().enumerate() {
                // Simulate filename formatting (replace {:04} with index)
                let output_name = filename_pattern.replace("{:04}", &format!("{:04}", idx + 1));
                echo(
                    &format!("Saving '{}' as '{}'", image.filename, output_name),
                    true,
                    false,
                    None,
                );
            }
            Ok(())
        })
        .build()
}

/// Opens all images in an image viewer.
fn display_cmd(stream: ImageStream) -> Command {
    Command::new("display")
        .help("Opens all images in an image viewer (simulated).")
        .callback(move |_ctx| {
            let stream_guard = stream.lock().unwrap();
            for image in stream_guard.iter() {
                echo(
                    &format!(
                        "Displaying '{}' ({}x{})",
                        image.filename, image.width, image.height
                    ),
                    true,
                    false,
                    None,
                );
            }
            Ok(())
        })
        .build()
}

/// Resizes an image by fitting it into a box without changing the aspect ratio.
fn resize_cmd(stream: ImageStream) -> Command {
    Command::new("resize")
        .help("Resizes an image by fitting it into the box without changing the aspect ratio.")
        .option(
            ClickOption::new(&["-w", "--width"])
                .help("The new width of the image.")
                .build(),
        )
        .option(
            ClickOption::new(&["-h", "--height"])
                .help("The new height of the image.")
                .build(),
        )
        .callback(move |ctx| {
            let width: Option<u32> = ctx
                .get_param::<String>("width")
                .and_then(|s| s.parse().ok());
            let height: Option<u32> = ctx
                .get_param::<String>("height")
                .and_then(|s| s.parse().ok());

            let mut stream_guard = stream.lock().unwrap();
            for image in stream_guard.iter_mut() {
                let new_w = width.unwrap_or(image.width);
                let new_h = height.unwrap_or(image.height);

                // Calculate aspect-ratio preserving dimensions
                let aspect = image.width as f64 / image.height as f64;
                let (final_w, final_h) = if new_w as f64 / new_h as f64 > aspect {
                    ((new_h as f64 * aspect) as u32, new_h)
                } else {
                    (new_w, (new_w as f64 / aspect) as u32)
                };

                echo(
                    &format!("Resizing '{}' to {}x{}", image.filename, final_w, final_h),
                    true,
                    false,
                    None,
                );
                image.width = final_w;
                image.height = final_h;
            }
            Ok(())
        })
        .build()
}

/// Crops an image from all edges.
fn crop_cmd(stream: ImageStream) -> Command {
    Command::new("crop")
        .help("Crops an image from all edges.")
        .option(
            ClickOption::new(&["-b", "--border"])
                .help("Crop the image from all sides by this amount.")
                .build(),
        )
        .callback(move |ctx| {
            let border: Option<u32> = ctx
                .get_param::<String>("border")
                .and_then(|s| s.parse().ok());

            if let Some(b) = border {
                let mut stream_guard = stream.lock().unwrap();
                for image in stream_guard.iter_mut() {
                    let new_w = image.width.saturating_sub(b * 2);
                    let new_h = image.height.saturating_sub(b * 2);
                    echo(
                        &format!(
                            "Cropping '{}' by {}px (new size: {}x{})",
                            image.filename, b, new_w, new_h
                        ),
                        true,
                        false,
                        None,
                    );
                    image.width = new_w.max(1);
                    image.height = new_h.max(1);
                }
            }
            Ok(())
        })
        .build()
}

/// Applies gaussian blur.
fn blur_cmd(stream: ImageStream) -> Command {
    Command::new("blur")
        .help("Applies gaussian blur.")
        .option(
            ClickOption::new(&["-r", "--radius"])
                .default("2")
                .help("The blur radius.")
                .build(),
        )
        .callback(move |ctx| {
            let radius: u32 = ctx
                .get_param::<String>("radius")
                .and_then(|s| s.parse().ok())
                .unwrap_or(2);

            let stream_guard = stream.lock().unwrap();
            for image in stream_guard.iter() {
                echo(
                    &format!("Blurring '{}' by {}px", image.filename, radius),
                    true,
                    false,
                    None,
                );
            }
            Ok(())
        })
        .build()
}

/// Sharpens an image.
fn sharpen_cmd(stream: ImageStream) -> Command {
    Command::new("sharpen")
        .help("Sharpens an image.")
        .option(
            ClickOption::new(&["-f", "--factor"])
                .default("2.0")
                .help("Sharpens the image.")
                .build(),
        )
        .callback(move |ctx| {
            let factor: f64 = ctx
                .get_param::<String>("factor")
                .and_then(|s| s.parse().ok())
                .unwrap_or(2.0);

            let stream_guard = stream.lock().unwrap();
            for image in stream_guard.iter() {
                echo(
                    &format!("Sharpening '{}' by factor {}", image.filename, factor),
                    true,
                    false,
                    None,
                );
            }
            Ok(())
        })
        .build()
}

/// Embosses an image.
fn emboss_cmd(stream: ImageStream) -> Command {
    Command::new("emboss")
        .help("Embosses an image.")
        .callback(move |_ctx| {
            let stream_guard = stream.lock().unwrap();
            for image in stream_guard.iter() {
                echo(
                    &format!("Embossing '{}'", image.filename),
                    true,
                    false,
                    None,
                );
            }
            Ok(())
        })
        .build()
}
