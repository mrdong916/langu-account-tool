use crate::domain::workspace::Bounds;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL};
use objc2_web_kit::WKWebView;
use tauri::{Url, Webview};

pub const BOOTSTRAP_PATH: &str = "/.langu-account-bootstrap";

pub fn set_bounds(view: &Webview, bounds: Bounds) -> tauri::Result<()> {
    // WKWebView's CSS viewport excludes the title bar safe area on recent macOS.
    // Child frames use the entire parent view, so translate using its current insets.
    view.with_webview(move |native| unsafe {
        let view = &*native.inner().cast::<WKWebView>();
        if let Some(parent) = view.superview() {
            let insets = parent.safeAreaInsets();
            let y = bounds.y + insets.top;
            let y = if parent.isFlipped() {
                y
            } else {
                parent.frame().size.height - y - bounds.height
            };
            view.setFrame(NSRect::new(
                NSPoint::new(bounds.x + insets.left, y),
                NSSize::new(bounds.width, bounds.height),
            ));
        }
    })
}

pub fn prepare_login(view: &Webview, mut landing: Url) -> tauri::Result<()> {
    landing.set_path(BOOTSTRAP_PATH);
    landing.set_query(None);
    landing.set_fragment(None);
    view.with_webview(move |native| unsafe {
        let view = &*native.inner().cast::<WKWebView>();
        let base = NSURL::URLWithString(&NSString::from_str(landing.as_str()));
        // An empty document in this account's own store and expected origin lets us
        // seed auth before any business router or lazy login component can clear it.
        // No payload is embedded in HTML or a persistent initialization script.
        view.loadHTMLString_baseURL(
            &NSString::from_str("<!doctype html><meta charset='utf-8'><title>正在登录</title>"),
            base.as_deref(),
        );
    })
}
