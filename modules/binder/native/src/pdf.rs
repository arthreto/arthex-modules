//! Turns a page into a PDF with `WebView2`, which Arthex itself needs, so it is always there. It runs
//! on a thread of its own, as `WebView2` wants a message loop, in a window nobody ever sees.

use std::{
    path::Path,
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use webview2_com::{
    CreateCoreWebView2ControllerCompletedHandler, CreateCoreWebView2EnvironmentCompletedHandler,
    Error,
    Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT, CreateCoreWebView2EnvironmentWithOptions,
        ICoreWebView2, ICoreWebView2_7, ICoreWebView2Controller, ICoreWebView2Environment,
        ICoreWebView2Environment6, ICoreWebView2PrintSettings,
    },
    NavigationCompletedEventHandler, PrintToPdfCompletedHandler, wait_with_pump,
};
use windows::{
    Win32::{
        Foundation::{E_FAIL, E_POINTER, E_UNEXPECTED, HWND, RECT},
        System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
        UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP},
    },
    core::{BOOL, HSTRING, Interface, PCWSTR, w},
};

/// In inches, as `WebView2` wants them: an A4 page.
const PAGE_WIDTH: f64 = 8.27;
const PAGE_HEIGHT: f64 = 11.69;
const MARGIN: f64 = 0.75;
/// A page with large images takes a while, but something is wrong well before this.
const LONGEST_WAIT: Duration = Duration::from_secs(120);

fn missing() -> windows::core::Error {
    windows::core::Error::from(E_POINTER)
}

fn unexpected<T>(_: T) -> windows::core::Error {
    windows::core::Error::from(E_UNEXPECTED)
}

fn create_environment(webview_data: &Path) -> Result<ICoreWebView2Environment, Error> {
    let (sender, receiver) = mpsc::channel();
    let data = HSTRING::from(webview_data.as_os_str());
    // SAFETY: the strings outlive the call, and the handler keeps what it needs.
    unsafe {
        CreateCoreWebView2EnvironmentWithOptions(
            PCWSTR::null(),
            &data,
            None,
            &CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
                move |result, environment| {
                    let answer = result.and_then(|()| environment.ok_or_else(missing));
                    sender.send(answer).map_err(unexpected)
                },
            )),
        )?;
    }
    Ok(wait_with_pump(receiver)??)
}

fn create_controller(
    environment: &ICoreWebView2Environment,
    window: HWND,
) -> Result<ICoreWebView2Controller, Error> {
    let (sender, receiver) = mpsc::channel();
    // SAFETY: the caller keeps `window` alive for as long as the controller.
    unsafe {
        environment.CreateCoreWebView2Controller(
            window,
            &CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                move |result, controller| {
                    let answer = result.and_then(|()| controller.ok_or_else(missing));
                    sender.send(answer).map_err(unexpected)
                },
            )),
        )?;
    }
    Ok(wait_with_pump(receiver)??)
}

fn load(webview: &ICoreWebView2, page: &Path) -> Result<(), Error> {
    let (sender, receiver) = mpsc::channel();
    let mut token = 0;
    let address = format!("file:///{}", page.to_string_lossy().replace('\\', "/"));
    // SAFETY: plain calls on a live object, with values that live through them.
    unsafe {
        // The page is ours and has no script, so nothing is lost if one ever slips in.
        webview.Settings()?.SetIsScriptEnabled(false)?;
        webview.add_NavigationCompleted(
            &NavigationCompletedEventHandler::create(Box::new(move |_, arguments| {
                let mut success = BOOL(0);
                if let Some(arguments) = arguments {
                    arguments.IsSuccess(&raw mut success)?;
                }
                sender.send(success.as_bool()).map_err(unexpected)
            })),
            &raw mut token,
        )?;
        webview.Navigate(&HSTRING::from(address))?;
    }
    if wait_with_pump(receiver)? {
        Ok(())
    } else {
        Err(windows::core::Error::from(E_FAIL).into())
    }
}

fn page_settings(
    environment: &ICoreWebView2Environment,
) -> Result<ICoreWebView2PrintSettings, Error> {
    // SAFETY: plain calls on live objects.
    unsafe {
        let settings = environment
            .cast::<ICoreWebView2Environment6>()?
            .CreatePrintSettings()?;
        settings.SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)?;
        settings.SetPageWidth(PAGE_WIDTH)?;
        settings.SetPageHeight(PAGE_HEIGHT)?;
        settings.SetMarginTop(MARGIN)?;
        settings.SetMarginBottom(MARGIN)?;
        settings.SetMarginLeft(MARGIN)?;
        settings.SetMarginRight(MARGIN)?;
        settings.SetShouldPrintBackgrounds(true)?;
        settings.SetShouldPrintHeaderAndFooter(false)?;
        Ok(settings)
    }
}

fn save_pdf(
    webview: &ICoreWebView2,
    settings: &ICoreWebView2PrintSettings,
    output: &Path,
) -> Result<(), Error> {
    let (sender, receiver) = mpsc::channel();
    // SAFETY: the strings and settings outlive the call, and the handler keeps what it needs.
    unsafe {
        webview.cast::<ICoreWebView2_7>()?.PrintToPdf(
            &HSTRING::from(output.as_os_str()),
            settings,
            &PrintToPdfCompletedHandler::create(Box::new(move |result, success| {
                let answer = result.and_then(|()| {
                    if success {
                        Ok(())
                    } else {
                        Err(windows::core::Error::from(E_FAIL))
                    }
                });
                sender.send(answer).map_err(unexpected)
            })),
        )?;
    }
    Ok(wait_with_pump(receiver)??)
}

fn print_in(window: HWND, page: &Path, output: &Path, webview_data: &Path) -> Result<(), Error> {
    let environment = create_environment(webview_data)?;
    let controller = create_controller(&environment, window)?;
    // SAFETY: plain calls on a live object.
    let webview = unsafe {
        controller.SetBounds(RECT {
            left: 0,
            top: 0,
            right: 800,
            bottom: 600,
        })?;
        controller.CoreWebView2()?
    };
    load(&webview, page)?;
    save_pdf(&webview, &page_settings(&environment)?, output)?;
    // SAFETY: nothing uses the controller any more.
    unsafe { controller.Close()? };
    Ok(())
}

fn print_page(page: &Path, output: &Path, webview_data: &Path) -> Result<(), String> {
    // SAFETY: this thread is ours and only runs this.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() }
        .map_err(|error| error.to_string())?;
    // SAFETY: a window of a system class, never shown and destroyed below.
    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            PCWSTR::null(),
            WS_POPUP,
            0,
            0,
            800,
            600,
            None,
            None,
            None,
            None,
        )
    }
    .map_err(|error| error.to_string())?;
    let result = print_in(window, page, output, webview_data).map_err(|error| error.to_string());
    // SAFETY: our own window, which nothing uses any more.
    let _ = unsafe { DestroyWindow(window) };
    result
}

/// Waits for at most `LONGEST_WAIT`: the overlay stays hidden until this answers.
pub fn print(page: &Path, output: &Path, webview_data: &Path) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    let (page, output, webview_data) =
        (page.to_owned(), output.to_owned(), webview_data.to_owned());
    thread::spawn(move || {
        let _ = sender.send(print_page(&page, &output, &webview_data));
    });
    match receiver.recv_timeout(LONGEST_WAIT) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err("the PDF took too long".to_string()),
        Err(RecvTimeoutError::Disconnected) => Err("the PDF could not be made".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    #[ignore = "needs WebView2, as on any computer running Arthex"]
    fn a_page_becomes_a_pdf() {
        let folder = tempfile::tempdir().unwrap();
        let page = folder.path().join("print.html");
        fs::write(
            &page,
            "<!doctype html><meta charset=utf-8><h1>Thermodynamique</h1><p>ΔU = Q + W</p>",
        )
        .unwrap();
        let output = folder.path().join("sortie.pdf");
        print(&page, &output, &folder.path().join("webview")).unwrap();
        let bytes = fs::read(&output).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "{:?}", &bytes[..8]);
    }
}
