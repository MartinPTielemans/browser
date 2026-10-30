//! A downloaded file's preview on Linux, as host_file_preview says: a photo's thumbnail, decoded by
//! the image crate GPUI decodes with, in the formats it has.
//!
//! shortcut: no video's first frame and no system icon for other files: those answer with no
//! preview, and the UI shows its own icon.

use std::{
    ffi::{CStr, c_char},
    io::Cursor,
    path::{Path, PathBuf},
};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

/// The longest side of a thumbnail, in pixels, as the Mac's.
const THUMBNAIL: u32 = 96;

type Done =
    extern "C" fn(id: u64, png: *const u8, size: usize, width: f64, height: f64, image: bool);

#[unsafe(no_mangle)]
pub extern "C" fn host_file_preview(path: *const c_char, id: u64, done: Done) {
    let path = PathBuf::from(unsafe { CStr::from_ptr(path) }.to_string_lossy().into_owned());
    let Some(cx) = crate::CX.with_borrow(Clone::clone) else {
        return;
    };
    // A photo's thumbnail reads and decodes its file, which can take a while for a large one.
    let thumbnail = cx.background_executor().spawn(async move { thumbnail(&path) });
    cx.foreground_executor()
        .spawn(async move {
            match thumbnail.await {
                Some((png, width, height)) => {
                    done(id, png.as_ptr(), png.len(), width.into(), height.into(), true)
                }
                None => done(id, std::ptr::null(), 0, 0., 0., false),
            }
        })
        .detach();
}

/// The picture at `path` as a PNG at most THUMBNAIL pixels on its longer side, upright as its
/// camera meant, with its width and height; none if it isn't a picture the image crate reads.
fn thumbnail(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    let reader = ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let orientation = decoder.orientation().ok()?;
    let mut picture = DynamicImage::from_decoder(decoder).ok()?;
    picture.apply_orientation(orientation);
    if picture.width().max(picture.height()) > THUMBNAIL {
        picture = picture.thumbnail(THUMBNAIL, THUMBNAIL);
    }
    let mut png = Vec::new();
    picture.write_to(&mut Cursor::new(&mut png), ImageFormat::Png).ok()?;
    Some((png, picture.width(), picture.height()))
}
