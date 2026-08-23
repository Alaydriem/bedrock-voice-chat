package com.alaydriem.bvc.client

import android.content.Context
import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  // Held so onResume can ask for a fresh inset dispatch. wry owns the view; this is a
  // reference to it, not ownership of it.
  private var insetWebView: WebView? = null

  // Resolved by Java_com_alaydriem_bvc_client_MainActivity_initNdkContext in
  // src-tauri/src/lib.rs. Tauri 2.11 / tao 0.35 stopped initializing the
  // global ndk-context (tauri-apps/tao#1154), so we populate it ourselves
  // with the application context before any plugin touches JNI on a worker
  // thread (cpal, tauri-plugin-keyring, webbrowser, ...).
  private external fun initNdkContext(context: Context)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    initNdkContext(this.applicationContext)
  }

  /**
   * Publish the window insets to the document as `--rad-inset-*`.
   *
   * wry makes the webview the activity's whole content view, and `enableEdgeToEdge` puts
   * that window behind the status bar and the navigation bar. Nothing in tao or wry reads
   * a window inset, so without this the app has no way to know where the system bars are.
   *
   * The webview's own `env(safe-area-inset-*)` is not enough on its own: Chromium reports
   * the system bars there only from M136, and only for a fullscreen webview until M144.
   * The webview is a Play Store component, so at our `minSdkVersion` a device can carry a
   * build older than either milestone and report 0 for a bar it is drawing under.
   *
   * This is the single source of the bar insets on Android: the bars are consumed below,
   * so Chromium reports 0 through `env()` and cannot apply them a second time. The `env()`
   * defaults the frontend declares (`client/src/css/shell.css`) are what iOS and iPadOS
   * use, where nothing native runs.
   */
  override fun onWebViewCreate(webView: WebView) {
    insetWebView = webView

    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, windowInsets ->
      val types =
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      val insets = windowInsets.getInsets(types)

      // Insets are physical pixels; a CSS pixel is one density unit.
      val density = webView.resources.displayMetrics.density.let { if (it > 0f) it else 1f }
      val top = (insets.top / density).toInt()
      val bottom = (insets.bottom / density).toInt()
      val left = (insets.left / density).toInt()
      val right = (insets.right / density).toInt()

      // `documentElement` because the tokens are declared on `:root`. An inline style
      // there outranks the stylesheet, which is what lets this win over the `env()`
      // default without either side knowing which components read the tokens.
      webView.evaluateJavascript(
        """
        (function (s) {
          s.setProperty('--rad-inset-top', '${top}px');
          s.setProperty('--rad-inset-bottom', '${bottom}px');
          s.setProperty('--rad-inset-left', '${left}px');
          s.setProperty('--rad-inset-right', '${right}px');
        })(document.documentElement.style);
        """.trimIndent(),
        null,
      )

      // The bars are consumed, and this is load-bearing. Handing them on as well leaves
      // Chromium applying the same inset a second time on top of the padding the write
      // above produces, which reads as a top bar with two status bars above it.
      //
      // Only the bars: `ime()` is left in place, so the keyboard still resizes the visual
      // viewport. Installing a listener replaces the view's own onApplyWindowInsets, so
      // the result still has to be handed to it or Chromium is cut out entirely.
      val withoutBars = WindowInsetsCompat.Builder(windowInsets)
        .setInsets(types, Insets.NONE)
        .build()

      ViewCompat.onApplyWindowInsets(view, withoutBars)
    }
  }

  override fun onResume() {
    super.onResume()
    requestInsets()
  }

  override fun onWindowFocusChanged(hasFocus: Boolean) {
    super.onWindowFocusChanged(hasFocus)
    if (hasFocus) requestInsets()
  }

  /**
   * Ask for another inset dispatch.
   *
   * The write in [onWebViewCreate] needs a document to write to, and the first dispatch can
   * land while the webview is still on `about:blank` — an inline style there does not
   * survive the navigation away from it. Because the bars are consumed there is no `env()`
   * fallback behind it on Android, so the write has to be re-driven rather than relied on
   * once.
   *
   * `onResume` runs before the first paint and `onWindowFocusChanged` after it, so between
   * them one request lands on a live document. Rotation, the keyboard and a navigation bar
   * mode change all dispatch again on their own.
   */
  private fun requestInsets() {
    insetWebView?.let { ViewCompat.requestApplyInsets(it) }
  }
}
