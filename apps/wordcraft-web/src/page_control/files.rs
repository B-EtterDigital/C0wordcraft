//! A cancellable request around the browser's existing file picker.
//! Cancelling retires the request; a late native chooser answer cannot replace a document.

use std::sync::atomic::Ordering;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use wordcraft_ui_egui::{WordApp, c0_host};

pub fn attach(app: &mut WordApp, ctx: &egui::Context) {
    let inbox = app.services.inbox.clone();
    let ctx = ctx.clone();
    app.services.open_async = Some(Box::new(move |purpose| {
        let Some(inbox) = inbox.clone() else { return };
        let pending = c0_host::begin_picker(&ctx);
        let ctx = ctx.clone();
        let filters = wordcraft_ui_egui::file_dialogs::open_filters(purpose);
        let many = purpose == "font";
        spawn_local(async move {
            let mut picker = rfd::AsyncFileDialog::new();
            for (label, extensions) in filters {
                picker = picker.add_filter(*label, extensions);
            }
            let files = if many { picker.pick_files().await.unwrap_or_default() } else { picker.pick_file().await.into_iter().collect() };
            if !pending.load(Ordering::Relaxed) {
                return;
            }
            let mut opened = Vec::new();
            let mut held = inbox.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|(_, b)| b.len()).fold(0, usize::saturating_add);
            for file in files.into_iter().take(super::super::web::MAX_PICKED_FILES) {
                // Bound the browser allocation before rfd reads the Blob, then verify its bytes.
                let size = file.inner().size();
                if !size.is_finite() || size < 0.0 || size > c0_host::MAX_FILE_BYTES.saturating_sub(held) as f64 {
                    log::warn!("page-control: picked files exceed the host file limit");
                    break;
                }
                let bytes = match JsFuture::from(file.inner().array_buffer()).await {
                    Ok(buffer) => js_sys::Uint8Array::new(&buffer).to_vec(),
                    Err(error) => {
                        log::warn!("page-control: could not read picked file: {error:?}");
                        continue;
                    }
                };
                if !pending.load(Ordering::Relaxed) {
                    return;
                }
                held = held.saturating_add(bytes.len());
                if held > c0_host::MAX_FILE_BYTES {
                    log::warn!("page-control: picked files exceed the host file limit");
                    break;
                }
                opened.push((file.file_name(), bytes));
            }
            if pending.swap(false, Ordering::Relaxed) {
                inbox.lock().unwrap_or_else(|e| e.into_inner()).extend(opened);
                ctx.request_repaint();
            }
        });
    }));
}
